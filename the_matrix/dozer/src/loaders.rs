use std::sync::Arc;

use bevy::prelude::*;
use mouse::sync::AsyncMutex;
use url::Url;

// pub struct DataLoader {
//    loaders: Vec<Arc<AsyncMutex<dyn AsyncReadSeek>>>,
//}
// pub struct SpawnLoader {
//    url: Url,
//}
// pub enum LoaderKind {
//    Hlcv {
//        mask: HlcvMask,
//        exhange: String,
//        market: String,
//    },
//}
// pub struct SpawnHlcvLoader {
//    pub mask: HlcvMask,
//    pub exhange: String,
//    pub market: String,
//}
// bitflags! {
//    #[derive(Default)]
//    pub struct HlcvMask: u8 {
//        const HIGH = 1 << 0;
//        const LOW = 1 << 1;
//        const CLOSE = 1 << 2;
//        const VOLUME = 1 << 3;
//    }
//}
// pub trait Loader {
//    fn spawn(url: Url) -> Result<Self>;
//}
//
//// open db connection
//// spawn dyn AsyncReadSeek
//// check if we have enough data
//// if not:
//// check if file is locked for writing
//// spawn downloader
//// if yes:
//// load block headers
//// load data by block
//// pipe to decompressor attach tag
//// forward to requestor
