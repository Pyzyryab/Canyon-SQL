use canyon_sql::*;
// use chrono::NaiveDate;
pub mod leagues;
pub mod tournaments;

use leagues::*;
use tournaments::*;

/// The `#[canyon]` macro represents the entry point of a Canyon managed program.
/// 
/// Go read the oficial docs for more info about the `#[canyon]` annotation (not docs yet)
/// 
/// TODO Docs explaining the virtues of `#[canyon]`, the `full managed state`
/// and the `just Crud operations` option
///  
#[canyon]  // TODO Add a log level argument
fn main() {
    /*  
        The insert example.
        On the first run, you may desire to uncomment the method call below,
        to be able to populate some data into the schema.
        Remember that all operation with CanyonCrud must be awaited,
        due to it's inherent async nature
    */
    // _wire_data_on_schema().await;

    /*
        The most basic usage pattern.
        Finds all elements on a type T, if the type its annotated with the
        #[derive(Debug, Clone, CanyonCrud, CanyonMapper)] derive macro

        This automatically returns a collection (Vector) of elements found
        after query the database, automatically desearializating the returning
        rows into elements of type T
    */
    let _all_leagues: Vec<League> = League::find_all().await;
    // println!("Leagues elements: {:?}", &_all_leagues);

    /*
        Canyon also has a powerful querybuilder.
        Every associated function or method provided through the macro implementations
        that returns a QueryBuilder type can be used as a raw builder to construct
        the query that Canyon will use to retrive data from the database.

        One really important thing to note it's that any struct annotated with the
        `[#canyon_entity]` annotation will generate and enumeration following the 
        convention: Type identifier + Fields holding variants to identify every
        field that the type has.

        So for a -> 
            pub struct League { /* fields */ }
        an enum with the fields as variants its generated ->
            pub enum LeagueFields { /* variants */ }

        So you must bring into scope `use::/* path to my type .rs file */::TypeFields`
        or simply `use::/* path to my type .rs file */::*` with a wildcard import.
        
        The querybuilder methods usually accept one of the variants of the enum to make a filter
        for the SQL clause, and a variant of the Canyon's `Comp` enum type, which indicates
        how the comparation element on the filter clauses will be 
    */
    let _all_leagues_as_querybuilder = League::find_all_query()
        .where_clause(
            LeagueFields::id(1), // This will create a filter -> `WHERE type.id = 1`
            Comp::Eq // where the `=` symbol it's given by this variant
        )
        .query()
        .await;
    // println!("Leagues elements QUERYBUILDER: {:?}", &_all_leagues_as_querybuilder);

    // Uncomment to see the example of find by a Fk relation
    _search_data_by_fk_example().await;
}

/// Example of usage of the `.insert()` Crud operation. Also, allows you
/// to wire some data on the database to be able to retrieve and play with data 
/// 
/// Notice how the `fn` must be `async`, due to Canyon's usage of **tokio**
/// as it's runtime
/// 
/// One big important note on Canyon insert. Canyon automatically manages
/// the ID field (commonly the primary key of any table) for you.
/// This means that if you keep calling this method, Canyon will keep inserting
/// records on the database, not with the id on the instance, only with the 
/// autogenerated one. 
/// 
/// This may change on a nearly time. 'cause it's direct implications on the
/// data integrity, but for now keep an eye on this.
/// 
/// An example of multiples inserts ignoring the provided `id` could end on a
/// situation like this:
/// 
/// ```
/// ... Leagues { id: 43, ext_id: 1, slug: "LEC", name: "League Europe Champions", region: "EU West", image_url: "https://lec.eu" }, 
/// Leagues { id: 44, ext_id: 2, slug: "LCK", name: "League Champions Korea", region: "South Korea", image_url: "https://korean_lck.kr" }, 
/// Leagues { id: 45, ext_id: 1, slug: "LEC", name: "League Europe Champions", region: "EU West", image_url: "https://lec.eu" }, 
/// Leagues { id: 46, ext_id: 2, slug: "LCK", name: "League Champions Korea", region: "South Korea", image_url: "https://korean_lck.kr" } ...
/// ``` 
async fn _wire_data_on_schema() {
    // Data for the examples
    let lec: League = League {
        id: 1,
        ext_id: 1,
        slug: "LEC".to_string(),
        name: "League Europe Champions".to_string(),
        region: "EU West".to_string(),
        image_url: "https://lec.eu".to_string(),
    };

    let lck: League = League {
        id: 2,
        ext_id: 2,
        slug: "LCK".to_string(),
        name: "League Champions Korea".to_string(),
        region: "South Korea".to_string(),
        image_url: "https://korean_lck.kr".to_string(),
    };

    // Now, the insert operations in Canyon is designed as a method over
    // the object, so the data of the instance is automatically parsed
    // into it's correct types and formats and inserted into the table
    lec.insert().await;
    lck.insert().await;

    /*  At some point on the console, if the operation it's successful, 
        you must see something similar to this, depending on the logging
        level choosed on Canyon
        
        INSERT STMT: INSERT INTO leagues (ext_id, slug, name, region, image_url) VALUES ($1,$2,$3,$4,$5)
        FIELDS: id, ext_id, slug, name, region, image_url

        INSERT STMT: INSERT INTO leagues (ext_id, slug, name, region, image_url) VALUES ($1,$2,$3,$4,$5)
        FIELDS: id, ext_id, slug, name, region, image_url
    */
}

/// Example of usage for a search given an entity related throught the 
/// `ForeignKey` annotation
/// 
/// Every struct that contains a `ForeignKey` annotation will have automatically
/// implemented a method to find data by an entity that it's related
/// through a foreign key relation.
/// 
/// So, in the example, the struct `Tournament` has a `ForeignKey` annotation
/// in it's `league` field, which holds a value relating the data on the `id` column
/// on the table `League`, so Canyon will generate an associated function following the convenction
/// `Type::search_by__name_of_the_related_table` 
/// 
/// TODO Upgrade DOCS according the two new ways of perform the fk search
async fn _search_data_by_fk_example() {
    // TODO Care with the docs. Split in two examples the two fk ways

    // TODO Explain that Canyon let's you annotate an entity with a FK but until a query, we 
    // can't no secure that the parent really exists
    // TODO Generate the inserts, updates and deletes with Foreign keys

    let tournament_itce = Tournament {
        id: 1,
        ext_id: 4126494859789,
        slug: "Slugaso".to_string(),
        league: 1,
    };
    let related_tournaments_league_method: Option<League> = tournament_itce.search_league().await;
    println!("The related League as method: {:?}", &related_tournaments_league_method);

    // Also, the common usage w'd be operating on data retrieve from the database, `but find_by_id`
    // returns an Option<T>, so an Option destructurement should be necessary
    let tournament: Option<Tournament> = Tournament::find_by_id(1).await;
    if let Some(trnmt) = tournament {
        let result: Option<League> = trnmt.search_league().await;
        println!("The related League as method if tournament is some: {:?}", &result);
    } else { println!("`tournament` variable contains a None value") }
    
    // The alternative as an associated function, passing as argument a type <K: ForeignKeyable> 
    // Data for the examples. Obviously will also work passing the above `tournament` variable as argument
    let lec: League = League {
        id: 4,
        ext_id: 1,
        slug: "LEC".to_string(),
        name: "League Europe Champions".to_string(),
        region: "EU West".to_string(),
        image_url: "https://lec.eu".to_string(),
    };
    let related_tournaments_league: Option<League> = Tournament::belongs_to(&lec).await;
    println!("The related League as associated function: {:?}", &related_tournaments_league);

    // TODO The reverse side of the FK should be implemented on League, not in tournament
    // EX: League::search_related__tournaments(&lec)
    // TODO Should be also an instance method? The lookage query w'd be based on the ID
    // like -> SELECT * FROM TOURNAMENT t WHERE t.league = (value of the field)
    let tournaments_belongs_to_league: Vec<Tournament> = Tournament::search_by__league(&lec).await;
    println!("Tournament belongs to a league: {:?}", &tournaments_belongs_to_league);

    // Method implementation over a League instance (prefered one)
    let tournaments_by_reverse_foreign: Vec<Tournament> = Tournament::search_by__league(&lec).await;
    println!("Tournament elements by reverse FK: {:?}", &tournaments_by_reverse_foreign);
}