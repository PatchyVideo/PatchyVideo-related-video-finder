
use std::{collections::HashSet, fmt, iter::FromIterator, u32};

use bson::{Document, doc, oid::ObjectId};
use futures_util::StreamExt;
use mongodb::{Collection, Database};
use rand_core::{RngCore, SeedableRng};
use serde::{Serialize, Deserialize};
use wyhash::{WyHash, WyRng};

#[derive(Debug, Clone)]
pub enum MinhashError {
}
impl std::error::Error for MinhashError {}

impl fmt::Display for MinhashError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntry {
	/// Bucket ID
	pub tid: i32,
	/// Hash Value
	pub val: i32,
	/// Corresponding Document ID
	pub did: ObjectId
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEntry {
	/// Corresponding Document ID
	pub did: ObjectId,
	/// Elements of the set
	pub eles: HashSet<i32>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnResultEntry {
	pub _id: ObjectId,
	pub data: DataEntry,
	pub count: i32
}

#[derive(Debug, Clone)]
pub struct MinhashDB {
	pub metadata_coll: Collection<MinhashOptions>,
	pub table_coll: Collection<TableEntry>,
	pub data_coll: Collection<DataEntry>,
	pub options: MinhashOptions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinhashOptions {
	pub num_hashes: i32,
	pub num_bands: Option<i32>,
	pub target_jaccard_similarity: Option<f32>
}

/// $ 1-(1-s^r)^b $
fn minhash_fn(rows: f32, bands: f32, x: f32) -> f32 {
	1.0 - (1.0 - x.powf(rows)).powf(bands)
}

/// $ \int_{a}^{b} {1-(1-s^r)^b} \,ds $
fn integrate_minhash_fn(rows: f32, bands: f32, a: f32, b: f32) -> f32 {
	let n: u32 = 512;
	let width = (b - a) / (n as f32);
	let mut ret: f32 = 0.0;
	for i in 0..n {
		let x1 = a + width * (i as f32);
		let x2 = a + width * ((i + 1) as f32);
		ret += (x2 - x1) / 6.0 * (minhash_fn(rows, bands, x1) + 4.0 * minhash_fn(rows, bands, (x1 + x2) * 0.5) + minhash_fn(rows, bands, x2));
	};
	ret
}

#[test]
fn test_integral() {
	let rows = 5.0;
	let bands = 20.0;
	assert!((integrate_minhash_fn(rows, bands, 0.0, 0.5) - 0.0446349).abs() < 1e-5);
	assert!((integrate_minhash_fn(rows, bands, 0.0, 0.9) - 0.39865).abs() < 1e-5);
	assert!((integrate_minhash_fn(rows, bands, 0.5, 1.0) - 0.454015).abs() < 1e-5);
}

impl MinhashOptions {
	pub fn calculate_bands(&mut self) {
		if let Some(s) = self.target_jaccard_similarity {
			let mut min_error = f32::MAX;
			let mut bands: i32 = 0;
			for b in 1..((self.num_hashes as f32).sqrt() as i32) {
				if self.num_hashes % b == 0 {
					let cur_error = integrate_minhash_fn((self.num_hashes / b) as f32, b as f32, 0.0, s) - 
					integrate_minhash_fn((self.num_hashes / b) as f32, b as f32, s, 1.0);
					if cur_error < min_error {
						min_error = cur_error;
						bands = b;
					}
				}
			}
			self.num_bands.get_or_insert(bands);
		}
	}
}

impl Default for MinhashOptions {
	fn default() -> Self {
		Self {
			num_hashes: 100,
			num_bands: None,
			target_jaccard_similarity: Some(0.7)
		}
	}
}
use std::hash::{Hash, Hasher};

fn insertion_sort_descending<T: PartialOrd>(s: &mut [T]) {
    for i in 1..s.len() {
        let mut j = i;
        while j > 0 && s[j - 1] < s[j] {
            s.swap(j - 1, j);
            j -= 1;
        }
    }
}


impl MinhashDB {
	fn calc_minhash(eles: Vec<i32>, num_hashes: i32) -> Vec<i32> {
		let mut hashers = eles.iter().map(|&item| WyRng::seed_from_u64(item as u64)).collect::<Vec<_>>();
		(0..num_hashes)
			.map(|_| {
				hashers
					.iter_mut()
					.map(|hasher| hasher.next_u32() as i32)
					.min()
					.expect("Expected non-zero hashers")
			})
			.collect()
	}
	fn to_band(v: &Vec<i32>, num_band: i32) -> Vec<i32> {
		let r = v.len() / num_band as usize;
		v.chunks(r).map(|chunk| Self::calculate_hash(&chunk)).collect()
	}
	fn calculate_hash<T: Hash>(t: &T) -> i32 {
		let mut hasher = WyHash::with_seed(1145141919810);
		t.hash(&mut hasher);
		hasher.finish() as i32
	}
	fn jaccard_similarity<T: Hash + Sized + Eq>(a: &HashSet<T>, b: &HashSet<T>) -> f32 {
		let i = a.intersection(&b).count() as f32;
		let u = a.union(&b).count() as f32;
		return i / u;
	}
	pub async fn connect_or_create(name: &str, db: &Database, options: Option<MinhashOptions>) -> Result<MinhashDB, Box<dyn std::error::Error>> {
		let mut options = options.unwrap_or_default();
		let data_coll_name = format!("{}_data", name);
		let table_coll_name = format!("{}_table", name);
		let metadata_coll_name = format!("{}_metadata", name);
		let metadata_coll = db.collection_with_type::<MinhashOptions>(&metadata_coll_name);
		if let Some(opt) = metadata_coll.find_one(doc! {}, None).await? {
			options = opt;
		} else {
			// create new DB
			if options.num_bands.is_none() {
				options.calculate_bands();
			}
			metadata_coll.insert_one(options.clone(), None).await?;
		}
		Ok(MinhashDB {
			data_coll: db.collection_with_type::<DataEntry>(&data_coll_name),
			table_coll: db.collection_with_type::<TableEntry>(&table_coll_name),
			metadata_coll: metadata_coll,
			options: options
		})
	}
	pub async fn insert_or_update_raw(&self, document: ObjectId, elements: &[i32]) -> Result<Option<DataEntry>, Box<dyn std::error::Error>> {
		let bands = self.options.num_bands.unwrap();
		let minhashes = Self::calc_minhash(elements.into_iter().cloned().collect::<Vec<i32>>(), self.options.num_hashes);
		let band_hashes = Self::to_band(&minhashes, bands);
		let mut ret = None;
		if let Some(old) = self.data_coll.find_one(doc! { "did": document.clone() }, None).await? {
			self.data_coll.delete_one(doc! { "did": document.clone() }, None).await?;
			self.table_coll.delete_many(doc! { "did": document.clone() }, None).await?;
			ret = Some(old);
		};
		let data = DataEntry {
			did: document.clone(),
			eles: elements.to_vec().iter().cloned().collect()
		};
		self.data_coll.insert_one(data, None).await?;
		let tables = band_hashes.iter().enumerate().map(|(i, &h)| {
			TableEntry {
				tid: i as _,
				val: h,
				did: document.clone()
			}
		}).collect::<Vec<_>>();
		self.table_coll.insert_many(tables, None).await?;
		Ok(ret)
	}
	pub async fn insert_or_update_lines(&self, document: ObjectId, elements: &Vec<String>) -> Result<Option<DataEntry>, Box<dyn std::error::Error>> {
		let hashes: Vec<i32> = elements.iter().map(|f| Self::calculate_hash(f)).collect();
		self.insert_or_update_raw(document, &hashes).await
	}
	pub async fn delete(&self, document: ObjectId) -> Result<Option<DataEntry>, Box<dyn std::error::Error>> {
		if let Some(old) = self.data_coll.find_one(doc! { "did": document.clone() }, None).await? {
			self.data_coll.delete_one(doc! { "did": document.clone() }, None).await?;
			self.table_coll.delete_many(doc! { "did": document.clone() }, None).await?;
			Ok(Some(old))
		} else {
			Ok(None)
		}
	}
	pub async fn find_exact(&self, document: ObjectId) -> Result<Option<DataEntry>, Box<dyn std::error::Error>> {
		if let Some(old) = self.data_coll.find_one(doc! { "did": document.clone() }, None).await? {
			Ok(Some(old))
		} else {
			Ok(None)
		}
	}
	pub async fn find_ann_raw(&self, elements: &[i32], top_k: Option<i32>, threshold: Option<f32>) -> Result<Vec<(f32, ObjectId)>, Box<dyn std::error::Error>> {
		let bands = self.options.num_bands.unwrap();
		let minhashes = Self::calc_minhash(elements.into_iter().cloned().collect::<Vec<i32>>(), self.options.num_hashes);
		let band_hashes = Self::to_band(&minhashes, bands);
		let hash_matchers = band_hashes.iter().enumerate().map(
			|(tid, &val)| {
				doc! {"tid": tid as i32, "val": val}
			}
		).collect::<Vec<_>>();
		let mut aggregate_terms = vec![
			doc! {"$match": {"$or": hash_matchers}},
			doc! {"$group": {"_id": "$did", "count": {"$sum": 1}}},
			doc! {"$sort":  {"count": -1}},
		];
		if let Some(&k) = top_k.as_ref() {
			aggregate_terms.push(doc! {"$limit": k});
		};
		aggregate_terms.push(doc! {"$lookup": {"from": self.data_coll.name(), "as": "data", "localField": "_id", "foreignField": "did"}});
		aggregate_terms.push(doc! {"$unwind": "$data"});
		let mut cursor = self.table_coll.aggregate(aggregate_terms, None).await?;
		let mut ret = Vec::with_capacity(*top_k.as_ref().unwrap_or(&20) as usize);
		let k_thres = top_k.unwrap_or(i32::MAX) as usize;
		let target_set: HashSet<i32> = elements.into_iter().cloned().collect();
		let sim_thres = threshold.unwrap_or(-1.0);
		while let Some(result) = cursor.next().await {
			let r: AnnResultEntry = bson::from_document(result?)?;
			let sim = Self::jaccard_similarity(&target_set, &r.data.eles);
			if sim >= sim_thres {
				ret.push((sim, r._id.clone()));
			}
		};
		insertion_sort_descending(&mut ret);
		ret = ret[..std::cmp::min(k_thres, ret.len())].to_vec();

		// TODO: cache result

		Ok(ret)
	}
	pub async fn find_ann_lines(&self, elements: &Vec<String>, top_k: Option<i32>, threshold: Option<f32>) -> Result<Vec<(f32, ObjectId)>, Box<dyn std::error::Error>> {
		let hashes: Vec<i32> = elements.iter().map(|f| Self::calculate_hash(f)).collect();
		self.find_ann_raw(&hashes, top_k, threshold).await
	}
}
