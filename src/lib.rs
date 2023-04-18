///! The root crate of the `Canyon-SQL` project.
///
/// Here it's where all the available functionalities and features
/// reaches the top most level, grouping them and making them visible
/// through this crate, building the *public API* of the library
extern crate canyon_connection;
extern crate canyon_crud;
extern crate canyon_macros;
extern crate canyon_observer;

extern crate async_trait;

/// Reexported elements to the root of the public API
pub mod migrations {
    pub use canyon_observer::migrations::{handler, processor};
}

/// The top level reexport. Here we define the path to some really important
/// things in `Canyon-SQL`, like the `main` macro, the IT macro.
pub use canyon_macros::main;

/// Public API for the `Canyon-SQL` proc-macros, and for the external ones
pub mod macros {
    pub use async_trait::*;
    pub use canyon_macros::*;
}

/// Crud module serves to reexport the public elements of the `canyon_crud` crate,
/// exposing them through the public API
pub mod crud {
    pub use canyon_crud::bounds;
    pub use canyon_crud::crud::*;
    pub use canyon_crud::mapper::*;
    pub use canyon_crud::DatabaseType;
}

/// Re-exports the query elements from the `crud`crate
pub mod query {
    pub use canyon_crud::query_elements::operators;
    pub use canyon_crud::query_elements::{query::*, query_builder::*};
}

/// Reexport the available database clients within Canyon
pub mod db_clients {
    #[cfg(feature = "mssql")]
    pub use canyon_connection::tiberius;
    #[cfg(feature = "postgres")]
    pub use canyon_connection::tokio_postgres;
}

/// Reexport the needed runtime dependencies
pub mod runtime {
    pub use canyon_connection::futures;
    pub use canyon_connection::init_connections_cache;
    pub use canyon_connection::tokio;
    pub use canyon_connection::tokio_util;
    pub use canyon_connection::CANYON_TOKIO_RUNTIME;
}

/// Module for reexport the `chrono` crate with the allowed public and available types in Canyon
pub mod date_time {
    pub use canyon_crud::chrono::{
        DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc,
    };
}
