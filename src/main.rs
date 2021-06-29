
use std::hash::{Hash, Hasher};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, Result, web::{self, Json}};
use bson::doc;
use futures_util::StreamExt;
use minhash_database::MinhashOptions;
use mongodb::{Client, options::ClientOptions};
use colored::*;
use probabilistic_collections::similarity::ShingleIterator;
use wyhash::WyHash;

use crate::models::{InsertRequest, ServiceError};

pub mod context;
pub mod models;
pub mod pvgql_models;
pub mod minhash_database;

fn calculate_hash<T: Hash>(t: &T) -> i32 {
	let mut hasher = WyHash::with_seed(1145141919810);
	t.hash(&mut hasher);
	hasher.finish() as i32
}

fn insertion_sort<T: PartialOrd>(s: &mut [T]) {
    for i in 1..s.len() {
        let mut j = i;
        while j > 0 && s[j - 1] > s[j] {
            s.swap(j - 1, j);
            j -= 1;
        }
    }
}

async fn create_db(ctx: web::Data<context::AppContext>) -> impl Responder {
	let mut cursor = ctx.video_coll.find(doc! {}, None).await.unwrap();
	while let Some(v) = cursor.next().await {
		insert_or_update(ctx.clone(), Json(InsertRequest {vid: v.unwrap()._id})).await.unwrap();
	};
	"done"
}

async fn insert_or_update(ctx: web::Data<context::AppContext>, request: web::Json<models::InsertRequest>) -> Result<web::Json<models::InsertResponse>> {
	if let Some(vid_obj) = ctx.video_coll.find_one(doc! {"_id": request.vid.clone()}, None).await.unwrap() {
		let title = vid_obj.item.title;
		if title == "【已失效视频】" {
			return Ok(web::Json(models::InsertResponse {}));
		}
		let title = format!(" {} ", title);
		println!("insert: '{}'", title);
		let title_chars = title.chars().collect::<Vec<_>>();
		let shingles = ShingleIterator::new(3, title_chars.iter().collect());
		let elements = shingles.map(|item| calculate_hash(&item)).collect::<Vec<i32>>();
		ctx.video_title_db.insert_or_update_raw(request.vid.clone(), &elements).await.unwrap();
	} else {
		println!("{}", format!("Video with ID={} not found", request.vid).red());
	}
	Ok(web::Json(models::InsertResponse {}))
}

async fn query_ann(ctx: web::Data<context::AppContext>, request: web::Json<models::QueryRequest>) -> Result<web::Json<models::QueryResponse>, ServiceError> {
	if let Some(vid_obj) = ctx.video_coll.find_one(doc! {"_id": request.vid.clone()}, None).await.unwrap() {
		let title = vid_obj.item.title;
		let title = format!(" {} ", title);
		let title_chars = title.chars().collect::<Vec<_>>();
		let shingles = ShingleIterator::new(3, title_chars.iter().collect());
		let elements = shingles.map(|item| calculate_hash(&item)).collect::<Vec<i32>>();
		let items = ctx.video_title_db.find_ann_raw(&elements, None, request.threshold).await.unwrap();
		let items = items.iter().map(|(_, o)| o.clone()).collect::<Vec<_>>();
		let mut ret = Vec::with_capacity(items.len());
		let mut found_videos = ctx.db.collection_with_type::<pvgql_models::Video>("videos").find(doc! { "_id": {"$in": items} }, None).await.unwrap();
		while let Some(v) = found_videos.next().await {
			ret.push(v.unwrap());
		};
		if let Some(q) = request.sort_title {
			if q {
				insertion_sort(&mut ret);
				let mut i = 0;
				loop {
					if ret[i]._id == request.vid {
						break i;
					}
					i += 1;
				};
				i += 1;
				if i != ret.len() {
					ret.drain(0..i);
				}
			}
		}
		println!("query: '{}' => {} results", title, ret.len());
		Ok(web::Json(models::QueryResponse {
			videos: ret[0..std::cmp::min(request.top_k.map_or(i32::MAX, |k| k) as usize, ret.len())].to_vec()
		}))
	} else {
		Err(ServiceError::NotFound)
	}
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

	let client_options = ClientOptions::parse(context::MONGODB_URL).await.expect("Failed to parse MongoDB parameters");
	let client = Client::with_options(client_options).expect("Failed to connect to MongoDB");

	let db = client.database("patchyvideo");

	let ctx = context::AppContext {
		db: db.clone(),
		video_coll: db.collection_with_type::<models::PartialVideo>("videos"),
		video_title_db: minhash_database::MinhashDB::connect_or_create("video_title_minhash", &db, Some(MinhashOptions {
			num_hashes: 120,
			num_bands: Some(40),
			target_jaccard_similarity: None,
		})).await.unwrap()
	};
	HttpServer::new(move || {
		App::new().data(ctx.clone())
			.route("/insert", web::post().to(insert_or_update))
			.route("/query", web::post().to(query_ann))
			.route("/create", web::post().to(create_db))
	})
	.bind(("0.0.0.0", 5010))?
	.run()
	.await
}