use crate::constants;
use canyon_crud::{crud::Transaction, DatabaseType, DatasourceConfig};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use walkdir::WalkDir;

use super::register_types::CanyonRegisterEntity;

/// Convenient struct that contains the necessary data and operations to implement
/// the `Canyon Memory`.
///
/// Canyon Memory it's just a convenient way of relate the data of a Rust source
/// code file and the `CanyonEntity` (if so), helping Canyon to know what source
/// file contains a `#[canyon_entity]` annotation and restricting it to just one
/// annotated struct per file.
///
/// This limitation it's imposed by design. Canyon, when manages all the entities in
/// the user's source code, needs to know for future migrations the old data about a structure
/// and the new modified one.
///
/// For example, let's say that you have a:
/// ```
/// pub struct Person {
///    /* some fields */
/// }
/// ```
///
/// and you decided to modify it's Ident and change it to `Human`.
///
/// Canyon will take care about modifying the Database, and `ALTER TABLE` to edit the actual data for you,
/// but, if it's not able to get the data to know that the old one is `Person` and the new one it's `Human`.
/// it will simply drop the table (losing all your data) and creating a new table `Human`.
///
/// So, we decised to follow the next approach:
/// Every entity annotated with a `#[canyon_entity]` annotation will be related to only unique Rust source
/// code file. If we find more, Canyon will raise and error saying that it does not allows to having more than
/// one managed entity per source file.
///
/// Then, we will store the entities data in a special table only for Canyon, where we will create the relation
/// between the source file, the entity and it's fields and data.
///
/// So, if the user wants or needs to modify the data of it's entity, Canyon can secure that will perform the
/// correct operations because we can't "remember" how that entity was, and how it should be now, avoiding
/// potentially dangerous operations due to lack of knowing what entity relates with new data.
///
/// The `memory field` HashMap is made by the filepath as a key, and the struct's name as value
#[derive(Debug)]
pub struct CanyonMemory {
    pub memory: Vec<CanyonMemoryAnalyzer>,
    pub renamed_entities: HashMap<String, String>,
}

// Makes this structure able to make queries to the database
impl Transaction<Self> for CanyonMemory {}

impl CanyonMemory {
    /// Queries the database to retrieve internal data about the structures
    /// tracked by `CanyonSQL`
    #[cfg(not(cargo_check))]
    #[allow(clippy::nonminimal_bool)]
    pub async fn remember(
        datasource: &DatasourceConfig,
        canyon_entities: &[CanyonRegisterEntity<'_>],
    ) -> Self {
        // Creates the memory table if not exists
        Self::create_memory(&datasource.name, &datasource.get_db_type()).await;

        // Retrieve the last status data from the `canyon_memory` table
        let res = Self::query("SELECT * FROM canyon_memory", [], &datasource.name)
            .await
            .expect("Error querying Canyon Memory");

        // Manually maps the results
        let mut db_rows = Vec::new();
        #[cfg(feature = "tokio-postgres")]
        {
            let mem_results: &Vec<tokio_postgres::Row> = res.get_postgres_rows();
            for row in mem_results {
                let db_row = CanyonMemoryRow {
                    id: row.get::<&str, i32>("id"),
                    filepath: row.get::<&str, String>("filepath"),
                    struct_name: row.get::<&str, String>("struct_name").to_owned(),
                    declared_table_name: row.get::<&str, String>("declared_table_name").to_owned(),
                };
                db_rows.push(db_row);
            }
        }
        #[cfg(feature = "tiberius")]
        {
            let mem_results: &Vec<tiberius::Row> = res.get_tiberius_rows();
            for row in mem_results {
                let db_row = CanyonMemoryRow {
                    id: row.get::<i32, &str>("id").unwrap(),
                    filepath: row.get::<&str, &str>("filepath").unwrap().to_string(),
                    struct_name: row.get::<&str, &str>("struct_name").unwrap().to_string(),
                    declared_table_name: row
                        .get::<&str, &str>("declared_table_name")
                        .unwrap()
                        .to_string(),
                };
                db_rows.push(db_row);
            }
        }

        Self::populate_memory(datasource, canyon_entities, db_rows).await
    }

    async fn populate_memory(
        datasource: &DatasourceConfig,
        canyon_entities: &[CanyonRegisterEntity<'_>],
        db_rows: Vec<CanyonMemoryRow>,
    ) -> CanyonMemory {
        let mut mem = Self {
            memory: Vec::new(),
            renamed_entities: HashMap::new(),
        };
        Self::find_canyon_entity_annotated_structs(&mut mem, canyon_entities).await;

        let mut updates = Vec::new();

        for _struct in &mem.memory {
            // For every program entity detected
            let already_in_db = db_rows.iter().find(|el| {
                el.filepath == _struct.filepath
                    || el.struct_name == _struct.struct_name
                    || el.declared_table_name == _struct.declared_table_name
            });

            if let Some(old) = already_in_db {
                if !(old.filepath == _struct.filepath
                    && old.struct_name == _struct.struct_name
                    && old.declared_table_name == _struct.declared_table_name)
                {
                    updates.push(&old.struct_name);
                    let stmt = format!(
                        "UPDATE canyon_memory SET filepath = '{}', struct_name = '{}', declared_table_name = '{}' \
                                WHERE id = {}",
                        _struct.filepath, _struct.struct_name, _struct.declared_table_name, old.id
                    );
                    save_canyon_memory_query(stmt, &datasource.name);

                    // if the updated element is the struct name, we add it to the table_rename Hashmap
                    let rename_table = old.declared_table_name != _struct.declared_table_name;

                    if rename_table {
                        mem.renamed_entities.insert(
                            _struct.declared_table_name.to_string(), // The new one
                            old.declared_table_name.to_string(),     // The old one
                        );
                    }
                }
            }

            if already_in_db.is_none() {
                let stmt = format!(
                    "INSERT INTO canyon_memory (filepath, struct_name, declared_table_name) \
                        VALUES ('{}', '{}', '{}')",
                    _struct.filepath, _struct.struct_name, _struct.declared_table_name
                );
                save_canyon_memory_query(stmt, &datasource.name)
            }
        }

        // Deletes the records from canyon_memory, because they stopped to be tracked by Canyon
        for db_row in db_rows.iter() {
            if !mem
                .memory
                .iter()
                .any(|entity| entity.struct_name == db_row.struct_name)
                && !updates.contains(&&(db_row.struct_name))
            {
                save_canyon_memory_query(
                    format!(
                        "DELETE FROM canyon_memory WHERE struct_name = '{}'",
                        db_row.struct_name
                    ),
                    &datasource.name,
                );
            }
        }
        mem
    }

    /// Parses the Rust source code files to find the one who contains Canyon entities
    /// ie -> annotated with `#[canyon_entity]`
    #[cfg(not(cargo_check))]
    async fn find_canyon_entity_annotated_structs(
        &mut self,
        canyon_entities: &[CanyonRegisterEntity<'_>],
    ) {
        for file in WalkDir::new("./src")
            .into_iter()
            .filter_map(|file| file.ok())
        {
            if file.metadata().unwrap().is_file()
                && file.path().display().to_string().ends_with(".rs")
            {
                // Opening the source code file
                let contents =
                    fs::read_to_string(file.path()).expect("Something went wrong reading the file");

                let mut canyon_entity_macro_counter = 0;
                let mut struct_name = String::new();
                for line in contents.split('\n') {
                    if line.contains("#[") // separated checks for possible different paths
                        && line.contains("canyon_entity")
                        && !line.starts_with("//")
                    {
                        canyon_entity_macro_counter += 1;
                    }

                    let re = Regex::new(r#"\bstruct\s+(\w+)"#).unwrap();
                    if let Some(captures) = re.captures(line) {
                        struct_name.push_str(captures.get(1).unwrap().as_str());
                    }
                }

                // This limitation will be removed in future versions, when the memory
                // will be able to track every aspect of an entity
                match canyon_entity_macro_counter {
                    0 => (),
                    1 => {
                        let canyon_entity = canyon_entities
                            .iter()
                            .find(|ce| ce.entity_name == struct_name);
                        if let Some(c_entity) = canyon_entity {
                            self.memory.push(CanyonMemoryAnalyzer {
                                filepath: file.path().display().to_string().replace('\\', "/"),
                                struct_name: struct_name.clone(),
                                declared_table_name: c_entity.entity_db_table_name.to_string(),
                            })
                        }
                    }
                    _ => panic!(
                        "Canyon-SQL does not support having multiple structs annotated
                        with `#[canyon::entity]` on the same file when the migrations are enabled"
                    ),
                }
            }
        }
    }

    /// Generates, if not exists the `canyon_memory` table
    async fn create_memory(datasource_name: &str, database_type: &DatabaseType) {
        let query = match database_type {
            #[cfg(feature = "tokio-postgres")]
            DatabaseType::PostgreSql => constants::postgresql_queries::CANYON_MEMORY_TABLE,
            #[cfg(feature = "tiberius")]
            DatabaseType::SqlServer => constants::mssql_queries::CANYON_MEMORY_TABLE,
        };

        Self::query(query, [], datasource_name)
            .await
            .expect("Error creating the 'canyon_memory' table");
    }
}

fn save_canyon_memory_query(stmt: String, ds_name: &str) {
    use crate::CM_QUERIES_TO_EXECUTE;

    if CM_QUERIES_TO_EXECUTE.lock().unwrap().contains_key(ds_name) {
        CM_QUERIES_TO_EXECUTE
            .lock()
            .unwrap()
            .get_mut(ds_name)
            .unwrap()
            .push(stmt);
    } else {
        CM_QUERIES_TO_EXECUTE
            .lock()
            .unwrap()
            .insert(ds_name.to_owned(), vec![stmt]);
    }
}

/// Represents a single row from the `canyon_memory` table
#[derive(Debug)]
struct CanyonMemoryRow {
    id: i32,
    filepath: String,
    struct_name: String,
    declared_table_name: String,
}

/// Represents the data that will be serialized in the `canyon_memory` table
#[derive(Debug)]
pub struct CanyonMemoryAnalyzer {
    pub filepath: String,
    pub struct_name: String,
    pub declared_table_name: String,
}
