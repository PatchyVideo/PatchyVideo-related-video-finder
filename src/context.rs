
use mongodb::{Collection, Database};

use crate::{minhash_database::MinhashDB, models};

#[cfg(debug_assertions)]
pub const MONGODB_URL: &str = "mongodb://localhost:27017/";

#[cfg(not(debug_assertions))]
pub const MONGODB_URL: &str = "mongodb://db:27017/";


#[derive(Clone, Debug)]
pub struct AppContext {
    pub db: Database,
    pub video_coll: Collection<models::PartialVideo>,
    pub video_title_db: MinhashDB
}

