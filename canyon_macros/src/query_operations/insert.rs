use proc_macro2::TokenStream;
use quote::quote;

use crate::utils::macro_tokens::MacroTokens;

/// Generates the TokenStream for the _insert_result() CRUD operation
pub fn generate_insert_tokens(macro_data: &MacroTokens, table_schema_data: &String) -> TokenStream {
    let ty = macro_data.ty;

    // Retrieves the fields of the Struct as a collection of Strings, already parsed
    // the condition of remove the primary key if it's present and it's autoincremental
    let insert_columns = macro_data.get_column_names_pk_parsed().join(", ");

    // Returns a String with the generic $x placeholder for the query parameters.
    let placeholders = macro_data.placeholders_generator();

    // Retrieves the fields of the Struct
    let fields = macro_data.get_struct_fields();

    let insert_values = fields.iter().map(|ident| {
        quote! { &self.#ident }
    });
    let insert_values_cloned = insert_values.clone();

    let primary_key = macro_data.get_primary_key_annotation();

    let remove_pk_value_from_fn_entry = if let Some(pk_index) = macro_data.get_pk_index() {
        quote! { values.remove(#pk_index) }
    } else {
        quote! {}
    };

    let pk_ident_type = macro_data
        ._fields_with_types()
        .into_iter()
        .find(|(i, _t)| Some(i.to_string()) == primary_key);
    let insert_transaction = if let Some(pk_data) = &pk_ident_type {
        let pk_ident = &pk_data.0;
        let pk_type = &pk_data.1;

        quote! {
            #remove_pk_value_from_fn_entry;

            let stmt = format!(
                "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
                #table_schema_data,
                #insert_columns,
                #placeholders,
                #primary_key
            );

            let rows = <#ty as canyon_sql::crud::Transaction<#ty>>::query_for_rows(
                stmt,
                values,
                datasource_name
            ).await?;

            match rows {
                #[cfg(feature = "tokio-postgres")] Self::Postgres(mut v) => {
                    instance.#pk_ident = v
                        .get(idx)
                        .expect("Failed getting the returned IDs for a multi insert")
                        .get::<&str, #pk_type>(#primary_key);
                    Ok(())
                },
                #[cfg(feature = "tiberius")] Self::Tiberius(mut v) => {
                    instance.#pk_ident = v
                        .get(idx)
                        .expect("Failed getting the returned IDs for a multi insert")
                        .get::<#pk_type, &str>(#primary_key)
                        .expect("SQL Server primary key type failed to be set as value");
                    Ok(())
                },
                _ => panic!() // TODO remove when the generics will be refactored
            }
        }
    } else {
        quote! {
            let stmt = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                #table_schema_data,
                #insert_columns,
                #placeholders,
                #primary_key
            );

            <#ty as canyon_sql::crud::Transaction<#ty>>::query(
                stmt,
                values,
                datasource_name
            ).await?;

            Ok(())
        }
    };

    quote! {
        /// Inserts into a database entity the current data in `self`, generating a new
        /// entry (row), returning the `PRIMARY KEY` = `self.<pk_field>` with the specified
        /// datasource by it's `datasouce name`, defined in the configuration file.
        ///
        /// This `insert` operation needs a `&mut` reference. That's because typically,
        /// an insert operation represents *new* data stored in the database, so, when
        /// inserted, the database will generate a unique new value for the
        /// `pk` field, having a unique identifier for every record, and it will
        /// automatically assign that returned pk to `self.<pk_field>`. So, after the `insert`
        /// operation, you instance will have the correct value that is the *PRIMARY KEY*
        /// of the database row that represents.
        ///
        /// This operation returns a result type, indicating a possible failure querying the database.
        ///
        /// ## *Examples*
        ///```
        /// let mut lec: League = League {
        ///     id: Default::default(),
        ///     ext_id: 1,
        ///     slug: "LEC".to_string(),
        ///     name: "League Europe Champions".to_string(),
        ///     region: "EU West".to_string(),
        ///     image_url: "https://lec.eu".to_string(),
        /// };
        ///
        /// println!("LEC before: {:?}", &lec);
        ///
        /// let ins_result = lec.insert_result().await;
        ///
        /// Now, we can handle the result returned, because it can contains a
        /// critical error that may leads your program to panic
        /// if let Ok(_) = ins_result {
        ///     println!("LEC after: {:?}", &lec);
        /// } else {
        ///     eprintln!("{:?}", ins_result.err())
        /// }
        /// ```
        ///
        async fn insert<'a>(&mut self)
            -> Result<(), Box<dyn std::error::Error + Sync + std::marker::Send>>
        {
            let datasource_name = "";
            let mut values: Vec<&dyn canyon_sql::crud::bounds::QueryParameter<'_>> = vec![#(#insert_values),*];
            #insert_transaction
        }

        /// Inserts into a database entity the current data in `self`, generating a new
        /// entry (row), returning the `PRIMARY KEY` = `self.<pk_field>` with the specified
        /// datasource by it's `datasouce name`, defined in the configuration file.
        ///
        /// This `insert` operation needs a `&mut` reference. That's because typically,
        /// an insert operation represents *new* data stored in the database, so, when
        /// inserted, the database will generate a unique new value for the
        /// `pk` field, having a unique identifier for every record, and it will
        /// automatically assign that returned pk to `self.<pk_field>`. So, after the `insert`
        /// operation, you instance will have the correct value that is the *PRIMARY KEY*
        /// of the database row that represents.
        ///
        /// This operation returns a result type, indicating a possible failure querying the database.
        ///
        /// ## *Examples*
        ///```
        /// let mut lec: League = League {
        ///     id: Default::default(),
        ///     ext_id: 1,
        ///     slug: "LEC".to_string(),
        ///     name: "League Europe Champions".to_string(),
        ///     region: "EU West".to_string(),
        ///     image_url: "https://lec.eu".to_string(),
        /// };
        ///
        /// println!("LEC before: {:?}", &lec);
        ///
        /// let ins_result = lec.insert_result().await;
        ///
        /// Now, we can handle the result returned, because it can contains a
        /// critical error that may leads your program to panic
        /// if let Ok(_) = ins_result {
        ///     println!("LEC after: {:?}", &lec);
        /// } else {
        ///     eprintln!("{:?}", ins_result.err())
        /// }
        /// ```
        ///
        async fn insert_datasource<'a>(&mut self, datasource_name: &'a str)
            -> Result<(), Box<dyn std::error::Error + Sync + std::marker::Send>>
        {
            let mut values: Vec<&dyn canyon_sql::crud::bounds::QueryParameter<'_>> = vec![#(#insert_values_cloned),*];
            #insert_transaction
        }

    }
}

/// Generates the TokenStream for the __insert() CRUD operation, but being available
/// as a [`QueryBuilder`] object, and instead of being a method over some [`T`] type,
/// as an associated function for [`T`]
///
/// This, also lets the user to have the option to be able to insert multiple
/// [`T`] objects in only one query
pub fn generate_multiple_insert_tokens(
    macro_data: &MacroTokens,
    table_schema_data: &String,
) -> TokenStream {
    let ty = macro_data.ty;

    // Retrieves the fields of the Struct as continuous String
    let column_names = macro_data.get_struct_fields_as_strings();

    // Retrieves the fields of the Struct
    let fields = macro_data.get_struct_fields();

    let macro_fields = fields.iter().map(|field| quote! { &instance.#field });
    let macro_fields_cloned = macro_fields.clone();

    let pk = macro_data.get_primary_key_annotation().unwrap_or_default();

    let pk_ident_type = macro_data
        ._fields_with_types()
        .into_iter()
        .find(|(i, _t)| *i == pk);

    let multi_insert_transaction = if let Some(pk_data) = &pk_ident_type {
        let pk_ident = &pk_data.0;
        let pk_type = &pk_data.1;

        quote! {
            mapped_fields = #column_names
                .split(", ")
                .map( |column_name| format!("\"{}\"", column_name))
                .collect::<Vec<String>>()
                .join(", ");

            let mut split = mapped_fields.split(", ")
                .collect::<Vec<&str>>();

            let pk_value_index = split.iter()
                .position(|pk| *pk == format!("\"{}\"", #pk).as_str())
                .expect("Error. No primary key found when should be there");
            split.retain(|pk| *pk != format!("\"{}\"", #pk).as_str());
            mapped_fields = split.join(", ").to_string();

            let mut fields_placeholders = String::new();

            let mut elements_counter = 0;
            let mut values_counter = 1;
            let values_arr_len = final_values.len();

            for vector in final_values.iter_mut() {
                let mut inner_counter = 0;
                fields_placeholders.push('(');
                vector.remove(pk_value_index);

                for _value in vector.iter() {
                    if inner_counter < vector.len() - 1 {
                        fields_placeholders.push_str(&("$".to_owned() + &values_counter.to_string() + ","));
                    } else {
                        fields_placeholders.push_str(&("$".to_owned() + &values_counter.to_string()));
                    }

                    inner_counter += 1;
                    values_counter += 1;
                }

                elements_counter += 1;

                if elements_counter < values_arr_len {
                    fields_placeholders.push_str("), ");
                } else {
                    fields_placeholders.push(')');
                }
            }

            let stmt = format!(
                "INSERT INTO {} ({}) VALUES {} RETURNING {}",
                #table_schema_data,
                mapped_fields,
                fields_placeholders,
                #pk
            );

            let mut v_arr = Vec::new();
            for arr in final_values.iter() {
                for value in arr {
                    v_arr.push(*value)
                }
            }

            let result = <#ty as canyon_sql::crud::Transaction<#ty>>::query(
                stmt,
                v_arr,
                datasource_name
            ).await;

            match result {
                Ok(res) => {
                    match res {
                        #[cfg(feature = "tokio-postgres")] Self::Postgres(mut v) => {
                            for (idx, instance) in instances.iter_mut().enumerate() {
                                instance.#pk_ident = v
                                    .get(idx)
                                    .expect("Failed getting the returned IDs for a multi insert")
                                    .get::<&str, #pk_type>(#pk);
                            }

                            Ok(())
                        },
                        #[cfg(feature = "tiberius")] Self::Tiberius(mut v) => {
                            for (idx, instance) in instances.iter_mut().enumerate() {
                                instance.#pk_ident = v
                                    .get(idx)
                                    .expect("Failed getting the returned IDs for a multi insert")
                                    .get::<#pk_type, &str>(#pk)
                                    .expect("SQL Server primary key type failed to be set as value");
                            }

                            Ok(())
                        },
                        _ => panic!() // TODO remove when the generics will be refactored
                    }
                },
                Err(e) => Err(e)
            }
        }
    } else {
        quote! {
            mapped_fields = #column_names
                .split(", ")
                .map( |column_name| format!("\"{}\"", column_name))
                .collect::<Vec<String>>()
                .join(", ");

            let mut split = mapped_fields.split(", ")
                .collect::<Vec<&str>>();

            let mut fields_placeholders = String::new();

            let mut elements_counter = 0;
            let mut values_counter = 1;
            let values_arr_len = final_values.len();

            for vector in final_values.iter_mut() {
                let mut inner_counter = 0;
                fields_placeholders.push('(');

                for _value in vector.iter() {
                    if inner_counter < vector.len() - 1 {
                        fields_placeholders.push_str(&("$".to_owned() + &values_counter.to_string() + ","));
                    } else {
                        fields_placeholders.push_str(&("$".to_owned() + &values_counter.to_string()));
                    }

                    inner_counter += 1;
                    values_counter += 1;
                }

                elements_counter += 1;

                if elements_counter < values_arr_len {
                    fields_placeholders.push_str("), ");
                } else {
                    fields_placeholders.push(')');
                }
            }

            let stmt = format!(
                "INSERT INTO {} ({}) VALUES {}",
                #table_schema_data,
                mapped_fields,
                fields_placeholders
            );

            let mut v_arr = Vec::new();
            for arr in final_values.iter() {
                for value in arr {
                    v_arr.push(*value)
                }
            }

            let result = <#ty as canyon_sql::crud::Transaction<#ty>>::query(
                stmt,
                v_arr,
                datasource_name
            ).await;

            match result {
                Ok(res) => Ok(()),
                Err(e) => Err(e)
            }
        }
    };

    quote! {
        /// Inserts multiple instances of some type `T` into its related table.
        ///
        /// ```
        /// let mut new_league = League {
        ///     id: Default::default(),
        ///    ext_id: 392489032,
        ///     slug: "League10".to_owned(),
        ///     name: "League10also".to_owned(),
        ///     region: "Turkey".to_owned(),
        ///     image_url: "https://www.sdklafjsd.com".to_owned()
        /// };
        /// let mut new_league2 = League {
        ///     id: Default::default(),
        ///     ext_id: 392489032,
        ///     slug: "League11".to_owned(),
        ///     name: "League11also".to_owned(),
        ///     region: "LDASKJF".to_owned(),
        ///     image_url: "https://www.sdklafjsd.com".to_owned()
        /// };
        /// let mut new_league3 = League {
        ///     id: Default::default(),
        ///     ext_id: 9687392489032,
        ///     slug: "League3".to_owned(),
        ///     name: "3League".to_owned(),
        ///     region: "EU".to_owned(),
        ///     image_url: "https://www.lag.com".to_owned()
        /// };
        ///
        /// League::insert_multiple(
        ///     &mut [&mut new_league, &mut new_league2, &mut new_league3]
        /// ).await
        /// .ok();
        /// ```
        async fn multi_insert<'a>(instances: &'a mut [&'a mut #ty]) -> (
            Result<(), Box<dyn std::error::Error + Sync + std::marker::Send>>
        ) {
            use canyon_sql::crud::bounds::QueryParameter;
            let datasource_name = "";

            let mut final_values: Vec<Vec<&dyn QueryParameter<'_>>> = Vec::new();
            for instance in instances.iter() {
                let intermediate: &[&dyn QueryParameter<'_>] = &[#(#macro_fields),*];

                let mut longer_lived: Vec<&dyn QueryParameter<'_>> = Vec::new();
                for value in intermediate.into_iter() {
                    longer_lived.push(*value)
                }

                final_values.push(longer_lived)
            }

            let mut mapped_fields: String = String::new();

            #multi_insert_transaction
        }

        /// Inserts multiple instances of some type `T` into its related table with the specified
        /// datasource by it's `datasouce name`, defined in the configuration file.
        ///
        /// ```
        /// let mut new_league = League {
        ///     id: Default::default(),
        ///    ext_id: 392489032,
        ///     slug: "League10".to_owned(),
        ///     name: "League10also".to_owned(),
        ///     region: "Turkey".to_owned(),
        ///     image_url: "https://www.sdklafjsd.com".to_owned()
        /// };
        /// let mut new_league2 = League {
        ///     id: Default::default(),
        ///     ext_id: 392489032,
        ///     slug: "League11".to_owned(),
        ///     name: "League11also".to_owned(),
        ///     region: "LDASKJF".to_owned(),
        ///     image_url: "https://www.sdklafjsd.com".to_owned()
        /// };
        /// let mut new_league3 = League {
        ///     id: Default::default(),
        ///     ext_id: 9687392489032,
        ///     slug: "League3".to_owned(),
        ///     name: "3League".to_owned(),
        ///     region: "EU".to_owned(),
        ///     image_url: "https://www.lag.com".to_owned()
        /// };
        ///
        /// League::insert_multiple(
        ///     &mut [&mut new_league, &mut new_league2, &mut new_league3]
        /// ).await
        /// .ok();
        /// ```
        async fn multi_insert_datasource<'a>(instances: &'a mut [&'a mut #ty], datasource_name: &'a str) -> (
            Result<(), Box<dyn std::error::Error + Sync + std::marker::Send>>
        ) {
            use canyon_sql::crud::bounds::QueryParameter;

            let mut final_values: Vec<Vec<&dyn QueryParameter<'_>>> = Vec::new();
            for instance in instances.iter() {
                let intermediate: &[&dyn QueryParameter<'_>] = &[#(#macro_fields_cloned),*];

                let mut longer_lived: Vec<&dyn QueryParameter<'_>> = Vec::new();
                for value in intermediate.into_iter() {
                    longer_lived.push(*value)
                }

                final_values.push(longer_lived)
            }

            let mut mapped_fields: String = String::new();

            #multi_insert_transaction
        }
    }
}
