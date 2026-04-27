pub mod index;
pub mod types;

#[allow(unused_imports)]
pub use index::{
    open_or_create, reindex_all, reindex_project, search, SearchError, SearchIndex, SearchSchema,
};
#[allow(unused_imports)]
pub use types::{SearchHit, SearchKind};
