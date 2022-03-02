use std::fmt::{Debug, Formatter};
use std::ops::Deref;

use mouse::futures_util::future::BoxFuture;
use mouse::futures_util::stream::BoxStream;
use mouse::prelude::*;
use mouse::rayon::iter::Either;
use mouse::sync::Mutex;
use sqlx::database::HasStatement;
use sqlx::{Database, Describe, Execute, Executor, MySql, MySqlExecutor, MySqlPool, Pool};

#[derive(Debug)]
pub struct Db {
    pool: Pool<MySql>,
}

impl Db {
    pub async fn connect() -> sqlx::Result<Self> {
        Ok(Self {
            pool: MySqlPool::connect("mysql://user:pass@host/database").await?,
        })
    }
}

impl Clone for Db {
    /// Increases rc
    fn clone(&self) -> Self {
        Db {
            pool: self.pool.clone(),
        }
    }
}

impl<'c> Executor<'c> for &Db {
    type Database = MySql;

    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxStream<
        'e,
        std::result::Result<
            Either<<Self::Database as Database>::QueryResult, <Self::Database as Database>::Row>,
            sqlx::Error,
        >,
    >
    where
        'c: 'e,
        E: Execute<'q, Self::Database>,
    {
        self.pool.fetch_many(query)
    }

    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxFuture<'e, std::result::Result<Option<<Self::Database as Database>::Row>, sqlx::Error>>
    where
        'c: 'e,
        E: Execute<'q, Self::Database>,
    {
        self.pool.fetch_optional(query)
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as Database>::TypeInfo],
    ) -> BoxFuture<
        'e,
        std::result::Result<<Self::Database as HasStatement<'q>>::Statement, sqlx::Error>,
    >
    where
        'c: 'e,
    {
        self.pool.prepare_with(sql, parameters)
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> BoxFuture<'e, std::result::Result<Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        self.pool.describe(sql)
    }
}

// lazy_static! {
//    pub static ref CORE_DB: Mutex<Option<Pool<MySql>>> = Default::default();
//}
// use chrono::{DateTime, NaiveDateTime};
// use config::CONFIG;
// use macros::Insert;
// use mouse::macros::futures_util::StreamExt;
// use mouse::prelude::*;
// use mouse::time::IntoDateTime;
// use sqlx::postgres::PgPoolOptions;
// use sqlx::{FromRow, PgPool, Postgres, Result, *};
// use url::Url;
//
// lazy_static! {
//    pub static ref DB: TankDb = TankDb::new(&CONFIG.db).unwrap();
//}
//
//#[derive(FromRow, Insert)]
// pub struct ResearchBlock {
//    pub combination_pos: i64,
//    pub combination_min: i64,
//    pub combination_max: i64,
//    pub balances_offset: i32,
//    pub balances_min: f32,
//    pub balances_max: f32,
//    pub max_balances_offset: i32,
//    pub max_balances_min: f32,
//    pub max_balances_max: f32,
//    pub n_trades_offset: i32,
//    pub n_trades_min: i32,
//    pub n_trades_max: i32,
//}
//
//#[derive(FromRow, Insert, Clone)]
// pub struct HlcvBlock {
//    pub start_time: NaiveDateTime,
//    pub file_id: i32,
//    pub end_time: NaiveDateTime,
//    pub high_pos: i64,
//    pub low_offset: i32,
//    pub close_offset: i32,
//    pub volume_offset: i32,
//}
// pub struct FileCreated {
//    pub file_id: i32,
//    pub host_path: String,
//}
//
//#[derive(Debug)]
// pub struct ResearchResultBlockTable(pub String);
//
// pub struct TankDb {
//    pool: PgPool,
//}
// impl TankDb {
//    pub fn new(url: &str) -> Result<Self> {
//        let pool = PgPoolOptions::new().max_connections(5).connect_lazy(url)?;
//        Ok(Self { pool })
//    }
//
//    pub async fn get_or_create_file(&self, path: &str) -> Result<FileCreated> {
//        #[derive(FromRow)]
//        pub struct FileCreatedInner {
//            pub file_id: Option<i32>,
//            pub host_path: Option<String>,
//        }
//        query_as!(
//            FileCreatedInner,
//            "select * from get_or_create_file($1)",
//            path
//        )
//        .fetch_one(&self.pool)
//        .await
//        .map(|x| FileCreated {
//            file_id: x.file_id.unwrap(),
//            host_path: x.host_path.unwrap(),
//        })
//    }
//
//    pub async fn create_file(&self, path: &Url) -> Result<i32> {
//        query_scalar!("select * from create_file($1)", path.as_str())
//            .fetch_one(&self.pool)
//            .await
//            .map(|x| x.unwrap())
//    }
//
//    pub async fn load_file_id(&self, path: &Url) -> Result<Option<i32>> {
//        if path.host().is_none() {
//            query_scalar!("select * from load_file_id_for_user($1)", path.as_str())
//                .fetch_one(&self.pool)
//                .await
//        } else {
//            query_scalar!("select * from load_file_id($1)", path.as_str())
//                .fetch_one(&self.pool)
//                .await
//        }
//    }
//
//    pub async fn create_research_result_block_table(
//        &self,
//        file_id: i32,
//    ) -> Result<ResearchResultBlockTable> {
//        query_scalar!(
//            "select * from create_research_result_block_table($1)",
//            file_id
//        )
//        .fetch_one(&self.pool)
//        .await
//        .map(|x| ResearchResultBlockTable(x.unwrap()))
//    }
//
//    pub async fn load_research_blocks(
//        &self,
//        table: &ResearchResultBlockTable,
//    ) -> Result<Vec<ResearchBlock>> {
//        // prepared statements can only contain one statement and thus not prone to sql injection
//        // attacks
//        query_as::<_, ResearchBlock>(&format!(
//            "select * from {} order by combination_pos asc",
//            table.0
//        ))
//        .fetch_all(&self.pool)
//        .await
//    }
//
//    pub async fn insert_research_block(
//        &self,
//        table: &ResearchResultBlockTable,
//        block: &ResearchBlock,
//    ) -> Result<()> {
//        block.insert(&table.0, &self.pool).await?;
//        Ok(())
//    }
//
//    pub async fn insert_hlcv_block(&self, block: &HlcvBlock) -> Result<()> {
//        block.insert("hlcv_blocks", &self.pool).await?;
//        Ok(())
//    }
//
//    pub async fn load_hlcv_blocks(
//        &self,
//        file_id: i32,
//        start_ts: i64,
//        end_ts: i64,
//    ) -> Result<Vec<HlcvBlock>> {
//        query_as!(
//            HlcvBlock,
//            "select * from hlcv_blocks where file_id = $1 and start_time >= $2 and end_time <
//     $3",
//            file_id,
//            start_ts.into_date_time().naive_utc(),
//            end_ts.into_date_time().naive_utc(),
//        )
//        .fetch_all(&self.pool)
//        .await
//    }
//}
// async fn main() -> Result<()> {
//    let db = TankDb::new("postgres://postgres:dev@localhost/dev")?;
//    dbg!(
//        db.create_file(&Url::parse("file:///dev/null").unwrap())
//            .await /*        db.load_research_blocks(&ResearchResultBlockTable("
//                    * research_result_blocks_3".into()))            .await */
//    );
//    //    dbg!(db.create_research_result_block_table(3).await);
//
//    //    let mut stream = sqlx::query_as::<_, User>("SELECT * FROM files").fetch(&pool);
//    //
//    //    while let Some(item) = stream.next().await {
//    //        dbg!(item);
//    //    }
//
//    Ok(())
//}
//
//#[test]
// fn t_main() {
//    let mut rt = tokio::runtime::Runtime::new().unwrap();
//    rt.block_on(main()).unwrap();
//}
