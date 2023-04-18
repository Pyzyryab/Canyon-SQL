use std::fmt::Display;

use async_trait::async_trait;
use canyon_connection::canyon_database_connector::DatabaseConnection;
use canyon_connection::{get_database_connection, CACHED_DATABASE_CONN};

use crate::bounds::QueryParameter;
use crate::mapper::RowMapper;
use crate::query_elements::query_builder::{
    DeleteQueryBuilder, SelectQueryBuilder, UpdateQueryBuilder,
};
use crate::rows::CanyonRows;

/// This traits defines and implements a query against a database given
/// an statement `stmt` and the params to pass the to the client.
///
/// It returns a [`DatabaseResult`], which is the core Canyon type to wrap
/// the result of the query and, if the user desires,
/// automatically map it to an struct.
#[async_trait]
pub trait Transaction<T> {
    /// Performs a query against the targeted database by the selected or
    /// the defaulted datasource, wrapping the resultant collection of entities
    /// in [`super::rows::Rows`]
    async fn query<'a, S, Z>(
        stmt: S,
        params: Z,
        datasource_name: &'a str,
    ) -> Result<CanyonRows<T>, Box<(dyn std::error::Error + Sync + Send + 'static)>>
    where
        S: AsRef<str> + Display + Sync + Send + 'a,
        Z: AsRef<[&'a dyn QueryParameter<'a>]> + Sync + Send + 'a,
    {
        let mut guarded_cache = CACHED_DATABASE_CONN.lock().await;
        let database_conn = get_database_connection(datasource_name, &mut guarded_cache);

        match *database_conn {
            #[cfg(feature = "tokio-postgres")]
            DatabaseConnection::Postgres(_) => {
                postgres_query_launcher::launch::<T>(
                    database_conn,
                    stmt.to_string(),
                    params.as_ref(),
                )
                .await
            }
            #[cfg(feature = "tiberius")]
            DatabaseConnection::SqlServer(_) => {
                sqlserver_query_launcher::launch::<T, Z>(
                    database_conn,
                    &mut stmt.to_string(),
                    params,
                )
                .await
            }
        }
    }
}

/// *CrudOperations* it's the core part of Canyon-SQL.
///
/// Here it's defined and implemented every CRUD operation
/// that the user has available, just by deriving the `CanyonCrud`
/// derive macro when a struct contains the annotation.
///
/// Also, this traits needs that the type T over what it's generified
/// to implement certain types in order to work correctly.
///
/// The most notorious one it's the [`RowMapper<T>`] one, which allows
/// Canyon to directly maps database results into structs.
///
/// See it's definition and docs to see the implementations.
/// Also, you can find the written macro-code that performs the auto-mapping
/// in the *canyon_sql_root::canyon_macros* crates, on the root of this project.
#[async_trait]
pub trait CrudOperations<T>: Transaction<T>
where
    T: CrudOperations<T> + RowMapper<T>,
{
    async fn find_all<'a>() -> Result<Vec<T>, Box<(dyn std::error::Error + Send + Sync + 'static)>>;

    async fn find_all_datasource<'a>(
        datasource_name: &'a str,
    ) -> Result<Vec<T>, Box<(dyn std::error::Error + Send + Sync + 'static)>>;

    async fn find_all_unchecked<'a>() -> Vec<T>;

    async fn find_all_unchecked_datasource<'a>(datasource_name: &'a str) -> Vec<T>;

    fn select_query<'a>() -> SelectQueryBuilder<'a, T>;

    fn select_query_datasource(datasource_name: &str) -> SelectQueryBuilder<'_, T>;

    async fn count() -> Result<i64, Box<(dyn std::error::Error + Send + Sync + 'static)>>;

    async fn count_datasource<'a>(
        datasource_name: &'a str,
    ) -> Result<i64, Box<(dyn std::error::Error + Send + Sync + 'static)>>;

    async fn find_by_pk<'a>(
        value: &'a dyn QueryParameter<'a>,
    ) -> Result<Option<T>, Box<(dyn std::error::Error + Send + Sync + 'static)>>;

    async fn find_by_pk_datasource<'a>(
        value: &'a dyn QueryParameter<'a>,
        datasource_name: &'a str,
    ) -> Result<Option<T>, Box<(dyn std::error::Error + Send + Sync + 'static)>>;

    async fn insert<'a>(&mut self) -> Result<(), Box<dyn std::error::Error + Sync + Send>>;

    async fn insert_datasource<'a>(
        &mut self,
        datasource_name: &'a str,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>>;

    async fn multi_insert<'a>(
        instances: &'a mut [&'a mut T],
    ) -> Result<(), Box<(dyn std::error::Error + Send + Sync + 'static)>>;

    async fn multi_insert_datasource<'a>(
        instances: &'a mut [&'a mut T],
        datasource_name: &'a str,
    ) -> Result<(), Box<(dyn std::error::Error + Send + Sync + 'static)>>;

    async fn update(&self) -> Result<(), Box<dyn std::error::Error + Sync + Send>>;

    async fn update_datasource<'a>(
        &self,
        datasource_name: &'a str,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>>;

    fn update_query<'a>() -> UpdateQueryBuilder<'a, T>;

    fn update_query_datasource(datasource_name: &str) -> UpdateQueryBuilder<'_, T>;

    async fn delete(&self) -> Result<(), Box<dyn std::error::Error + Sync + Send>>;

    async fn delete_datasource<'a>(
        &self,
        datasource_name: &'a str,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>>;

    fn delete_query<'a>() -> DeleteQueryBuilder<'a, T>;

    fn delete_query_datasource(datasource_name: &str) -> DeleteQueryBuilder<'_, T>;
}

#[cfg(feature = "tokio-postgres")]
mod postgres_query_launcher {
    use crate::bounds::QueryParameter;
    use crate::rows::CanyonRows;
    use canyon_connection::canyon_database_connector::DatabaseConnection;

    pub async fn launch<'a, T>(
        db_conn: &DatabaseConnection,
        stmt: String,
        params: &'a [&'_ dyn QueryParameter<'_>],
    ) -> Result<CanyonRows<T>, Box<(dyn std::error::Error + Send + Sync + 'static)>> {
        let mut m_params = Vec::new();
        for param in params {
            m_params.push(param.as_postgres_param());
        }

        let r = db_conn
            .postgres_connection()
            .unwrap()
            .client
            .query(&stmt, m_params.as_slice())
            .await?;

        Ok(CanyonRows::Postgres(r))
    }
}

#[cfg(feature = "tiberius")]
mod sqlserver_query_launcher {
    use crate::rows::CanyonRows;
    use crate::{
        bounds::QueryParameter,
        canyon_connection::{canyon_database_connector::DatabaseConnection, tiberius::Query},
    };

    pub async fn launch<'a, T, Z>(
        db_conn: &mut DatabaseConnection,
        stmt: &mut String,
        params: Z,
    ) -> Result<CanyonRows<T>, Box<(dyn std::error::Error + Send + Sync + 'static)>>
    where
        Z: AsRef<[&'a dyn QueryParameter<'a>]> + Sync + Send + 'a,
    {
        // Re-generate de insert statement to adequate it to the SQL SERVER syntax to retrieve the PK value(s) after insert
        if stmt.contains("RETURNING") {
            let c = stmt.clone();
            let temp = c.split_once("RETURNING").unwrap();
            let temp2 = temp.0.split_once("VALUES").unwrap();

            *stmt = format!(
                "{} OUTPUT inserted.{} VALUES {}",
                temp2.0.trim(),
                temp.1.trim(),
                temp2.1.trim()
            );
        }

        let mut mssql_query = Query::new(stmt.to_owned().replace('$', "@P"));
        params
            .as_ref()
            .iter()
            .for_each(|param| mssql_query.bind(*param));

        let _results = mssql_query
            .query(
                db_conn
                    .sqlserver_connection()
                    .expect("Error querying the MSSQL database")
                    .client,
            )
            .await?
            .into_results()
            .await?;

        Ok(CanyonRows::Tiberius(
            _results.into_iter().flatten().collect(),
        ))
    }
}
