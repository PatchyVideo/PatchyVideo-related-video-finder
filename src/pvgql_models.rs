
extern crate serde_json;
use std::{cell::RefMut, cmp::Ordering, fmt};

use serde::{Serialize, Deserialize};
use bson::oid::ObjectId;
use chrono::{DateTime, Utc};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Error {
	pub code: String,
	pub aux: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RestResultError {
	pub reason: String,
	pub aux: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RestResult<T> {
	pub status: String,
	pub data: Option<T>,
	pub dataerr: Option<RestResultError>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MyObjectId {
	Oid(serde_json::Map<String, serde_json::Value>),
	Str(String)
}

impl MyObjectId {
	pub fn to_oid(&self) -> Option<ObjectId> {
		match self {
			MyObjectId::Oid(o) => {
				match o.get("$oid") {
					Some(value) => {
						match value.as_str() {
							Some(s) => {
								match ObjectId::with_string(s) {
									Ok(oid) => Some(oid),
									Err(_) => None
								}
							}
							None => None
						}
					},
					None => None
				}
			},
			MyObjectId::Str(s) => {
				if s.len() > 0 {
					match ObjectId::with_string(s) {
						Ok(oid) => Some(oid),
						Err(_) => None
					}
				} else {
					None
				}
			}
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
	pub created_at: bson::DateTime,
	pub created_by: Option<MyObjectId>,
	pub modified_at: Option<bson::DateTime>,
	pub modified_by: Option<MyObjectId>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BsonDateTime
{
	#[serde(rename = "$date")]
	pub ts: i64
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TagCategoryEnum {
	General,
	Character,
	Copyright,
	Author,
	Meta,
	Language,
	Soundtrack
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoItem {
	pub cover_image: String,
	pub title: String,
	pub desc: String,
	pub placeholder: Option<bool>,
	pub rating: f64,
	pub repost_type: String,
	pub copies: Vec<ObjectId>,
	pub series: Vec<ObjectId>,
	pub site: String,
	pub thumbnail_url: String,
	pub unique_id: String,
	pub upload_time: bson::DateTime,
	pub url: String,
	pub user_space_urls: Option<Vec<String>>,
	pub utags: Vec<String>,
	pub views: i32,
	pub cid: Option<u64>,
	pub part_name: Option<String>
}


#[derive(Clone, Serialize, Deserialize)]
pub struct TagCategoryItem {
	pub key: TagCategoryEnum,
	pub value: Vec<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
	pub _id: ObjectId,
	pub clearence: i32,
	pub item: VideoItem,
	pub meta: Meta,
	pub tag_count: i32,
	pub tags: Vec<i64>,
	pub comment_thread: Option<ObjectId>
}


#[derive(Clone, Serialize, Deserialize)]
pub struct MultilingualMapping {
	pub lang: String,
	pub value: String
}

impl PartialOrd for Video {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.item.title.cmp(&other.item.title))
    }
}
impl PartialEq for Video {
    fn eq(&self, other: &Self) -> bool {
        self.item.title == other.item.title
    }
}
