#![allow(clippy::extra_unused_lifetimes)]

use crate::{
    crud::{CrudOperations, Transaction},
    mapper::RowMapper,
};
use canyon_connection::{
    tiberius::{ColumnData, IntoSql, self},
    tokio_postgres::{types::ToSql, self}
};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use std::{fmt::Debug, any::Any};

/// Created for retrieve the field's name of a field of a struct, giving
/// the Canoyn's autogenerated enum with the variants that maps this
/// fields.
///
/// ```
/// pub struct Struct<'a> {
///     pub some_field: &'a str
/// }
///
/// // Autogenerated enum
/// #[derive(Debug)]
/// #[allow(non_camel_case_types)]
/// pub enum StructField {
///     some_field
/// }
/// ```
/// So, to retrieve the field's name, something like this w'd be used on some part
/// of the Canyon's Manager crate, to wire the necessary code to pass the field
/// name, retrieved from the enum variant, to a called.
///
/// // Something like:
/// `let struct_field_name_from_variant = StructField::some_field.field_name_as_str();`
pub trait FieldIdentifier<T>
where
    T: Transaction<T> + CrudOperations<T> + RowMapper<T> + Debug,
{
    fn field_name_as_str(self) -> String;
}

/// Represents some kind of introspection to make the implementors
/// retrieves a value inside some variant of an associated enum type.
/// and convert it to an [`String`], to enable the convertion of
/// that value into something that can be part of an SQL query.
///
/// It's a generification to convert everything to a string representation
/// in SQL syntax, so the clauses can use any value to make filters
///
/// Ex:
/// `SELECT * FROM some_table WHERE id = '2'`
///
/// That '2' it's extracted from some enum that implements [`FieldValueIdentifier`],
/// where usually the variant w'd be something like:
///
/// ```
/// pub enum Enum {
///     IntVariant(i32)
/// }
/// ```
/// so, the `.value(self)` method it's called over `self`, gets the value for that variant
/// (or another specified in the logic) and returns that value as an [`String`]
pub trait FieldValueIdentifier<T>
where
    T: Transaction<T> + CrudOperations<T> + RowMapper<T> + Debug,
{
    fn value(self) -> String;
}

impl<T> FieldValueIdentifier<T> for &str
where
    T: Transaction<T> + CrudOperations<T> + RowMapper<T> + Debug,
{
    fn value(self) -> String {
        self.to_string()
    }
}

// impl<T> FieldValueIdentifier<T> for Option<String>
// where
//     T: Transaction<T> + CrudOperations<T> + RowMapper<T> + Debug,
// {
//     fn value(self) -> String {
//         self.unwrap().to_string()
//     }
// }

/// Bounds to some type T in order to make it callable over some fn parameter T
///
/// Represents the ability of an struct to be considered as candidate to perform
/// actions over it as it holds the 'parent' side of a foreign key relation.
///
/// Usually, it's used on the Canyon macros to retrieve the column that
/// this side of the relation it's representing
pub trait ForeignKeyable<T> {
    /// Retrieves the field related to the column passed in
    fn get_fk_column(&self, column: &str) -> Option<&dyn QueryParameters<'_>>;
}

/// To define trait objects that helps to relates the necessary bounds in the 'IN` SQL clause
pub trait InClauseValues: ToSql + ToString {}

/// Generic abstraction to represent any of the Row types
/// from the client crates
pub trait Row {
    fn as_any(&self) -> &dyn Any;
    
}
impl Row for tokio_postgres::Row {
    fn as_any(&self) -> &dyn Any { self }
}

impl Row for tiberius::Row {
    fn as_any(&self) -> &dyn Any { self }
}

pub struct Column<'a> {
    name: &'a str,
    type_: ColumnType
}
impl<'a> Column<'a> {
    pub fn name(&self) -> &'_ str {
        self.name
    }
    pub fn column_type(&self) -> &ColumnType {
        &self.type_
    }
    pub fn type_(&'a self) -> &'_ dyn Type {
        match &self.type_ {
            ColumnType::Postgres(v) => v as &'a dyn Type,
            ColumnType::SqlServer(v) => v as &'a dyn Type,
        }
    }
}


pub trait Type {
    fn as_any(&self) -> &dyn Any;
}
impl Type for tokio_postgres::types::Type {
    fn as_any(&self) -> &dyn Any { self }
}
impl Type for tiberius::ColumnType {
    fn as_any(&self) -> &dyn Any { self }
}

pub enum ColumnType {
    Postgres(tokio_postgres::types::Type),
    SqlServer(tiberius::ColumnType)
}

pub trait RowOperations {
    /// Abstracts the different forms of use the common `get` row
    /// function or method dynamically no matter what are the origin
    /// type from any database client provider
    fn get<'a, Output>(&'a self, col_name: &str) -> Output
        where Output: tokio_postgres::types::FromSql<'a> + tiberius::FromSql<'a>;

    fn get_opt<'a, Output>(&'a self, col_name: &str) -> Option<Output> 
        where Output: tokio_postgres::types::FromSql<'a> + tiberius::FromSql<'a>;

    fn columns<'a>(&'a self) -> Vec<Column>;
}

impl RowOperations for &dyn Row {
    fn get<'a, Output>(&'a self, col_name: &str) -> Output 
        where Output: tokio_postgres::types::FromSql<'a>  + tiberius::FromSql<'a>
    {
        match self.as_any().downcast_ref::<tokio_postgres::Row>() {
            Some(row) => { return row.get::<&str, Output>(col_name); },
            None => (),
        };
        match self.as_any().downcast_ref::<tiberius::Row>() {
            Some(row) => { 
                return row.get::<Output, &str>(col_name)
                    .expect("Failed to obtain a row in the MSSQL migrations"); 
            },
            None => (),
        };
        panic!()
    }

    fn columns<'a>(&'a self) -> Vec<Column>
    {
        let mut cols = vec![];

        if self.as_any().is::<tokio_postgres::Row>() {
            self.as_any().downcast_ref::<tokio_postgres::Row>()
                .expect("Not a tokio postgres Row for column")
                .columns()
                .into_iter()
                .for_each(|c| cols.push(
                    Column {
                        name: c.name(),
                        type_: ColumnType::Postgres(c.type_().to_owned())
                    }
                ))
        } else {
            self.as_any().downcast_ref::<tiberius::Row>()
                .expect("Not a Tiberius Row for column")
                .columns()
                .into_iter()
                .for_each(|c| cols.push(
                    Column {
                        name: c.name(),
                        type_: ColumnType::SqlServer(c.column_type())
                    }
                ))
        };

        cols
    }

    fn get_opt<'a, Output>(&'a self, col_name: &str) -> Option<Output> 
        where Output: tokio_postgres::types::FromSql<'a> + tiberius::FromSql<'a> 
    {
        match self.as_any().downcast_ref::<tokio_postgres::Row>() {
            Some(row) => { return row.get::<&str, Option<Output>>(col_name); },
            None => (),
        };
        match self.as_any().downcast_ref::<tiberius::Row>() {
            Some(row) => { 
                return row.try_get::<Output, &str>(col_name)
                    .expect("Failed to obtain a row in the MSSQL migrations"); 
            },
            None => (),
        };
        panic!()
    }
}


/// Defines a trait for represent type bounds against the allowed
/// datatypes supported by Canyon to be used as query parameters.
pub trait QueryParameters<'a>: std::fmt::Debug + Sync + Send {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync);
    fn as_sqlserver_param(&self) -> ColumnData<'_>;
}

/// The implementation of the [`canyon_connection::tiberius`] [`IntoSql`] for the
/// query parameters.
///
/// This implementation is necessary because of the generic amplitude
/// of the arguments of the [`Transaction::query`], that should work with
/// a collection of [`QueryParameters<'a>`], in order to allow a workflow
/// that is not dependant of the specific type of the argument that holds
/// the query parameters of the database connectors
impl<'a> IntoSql<'a> for &'a dyn QueryParameters<'a> {
    fn into_sql(self) -> ColumnData<'a> {
        self.as_sqlserver_param()
    }
}

impl<'a> QueryParameters<'a> for i16 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I16(Some(*self))
    }
}
impl<'a> QueryParameters<'a> for &i16 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I16(Some(**self))
    }
}
impl<'a> QueryParameters<'a> for Option<i16> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I16(*self)
    }
}
impl<'a> QueryParameters<'a> for Option<&i16> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I16(Some(*self.unwrap()))
    }
}
impl<'a> QueryParameters<'a> for i32 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I32(Some(*self))
    }
}
impl<'a> QueryParameters<'a> for &i32 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I32(Some(**self))
    }
}
impl<'a> QueryParameters<'a> for Option<i32> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I32(*self)
    }
}
impl<'a> QueryParameters<'a> for Option<&i32> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I32(Some(*self.unwrap()))
    }
}
impl<'a> QueryParameters<'a> for f32 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::F32(Some(*self))
    }
}
impl<'a> QueryParameters<'a> for &f32 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::F32(Some(**self))
    }
}
impl<'a> QueryParameters<'a> for Option<f32> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::F32(*self)
    }
}
impl<'a> QueryParameters<'a> for Option<&f32> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::F32(Some(
            *self.expect("Error on an f32 value on QueryParameters<'_>"),
        ))
    }
}
impl<'a> QueryParameters<'a> for f64 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::F64(Some(*self))
    }
}
impl<'a> QueryParameters<'a> for &f64 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::F64(Some(**self))
    }
}
impl<'a> QueryParameters<'a> for Option<f64> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::F64(*self)
    }
}
impl<'a> QueryParameters<'a> for Option<&f64> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::F64(Some(
            *self.expect("Error on an f64 value on QueryParameters<'_>"),
        ))
    }
}
impl<'a> QueryParameters<'a> for i64 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I64(Some(*self))
    }
}
impl<'a> QueryParameters<'a> for &i64 {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I64(Some(**self))
    }
}
impl<'a> QueryParameters<'a> for Option<i64> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I64(*self)
    }
}
impl<'a> QueryParameters<'a> for Option<&i64> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::I64(Some(*self.unwrap()))
    }
}
impl<'a> QueryParameters<'a> for String {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::String(Some(std::borrow::Cow::Owned(self.to_owned())))
    }
}
impl<'a> QueryParameters<'a> for &String {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::String(Some(std::borrow::Cow::Borrowed(self)))
    }
}
impl<'a> QueryParameters<'a> for Option<String> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        match self {
            Some(string) => ColumnData::String(Some(std::borrow::Cow::Owned(string.to_owned()))),
            None => ColumnData::String(None),
        }
    }
}
impl<'a> QueryParameters<'a> for Option<&String> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        match self {
            Some(string) => ColumnData::String(Some(std::borrow::Cow::Borrowed(string))),
            None => ColumnData::String(None),
        }
    }
}
impl<'a> QueryParameters<'_> for &'_ str {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        ColumnData::String(Some(std::borrow::Cow::Borrowed(*self)))
    }
}
impl<'a> QueryParameters<'a> for Option<&'_ str> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        match *self {
            Some(str) => ColumnData::String(Some(std::borrow::Cow::Borrowed(str))),
            None => ColumnData::String(None),
        }
    }
}
impl<'a> QueryParameters<'_> for NaiveDate {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'a> for Option<NaiveDate> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'_> for NaiveTime {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'a> for Option<NaiveTime> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'_> for NaiveDateTime {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'a> for Option<NaiveDateTime> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'_> for DateTime<FixedOffset> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'a> for Option<DateTime<FixedOffset>> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'_> for DateTime<Utc> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
impl<'a> QueryParameters<'_> for Option<DateTime<Utc>> {
    fn as_postgres_param(&self) -> &(dyn ToSql + Sync) {
        self
    }

    fn as_sqlserver_param(&self) -> ColumnData<'_> {
        self.into_sql()
    }
}
