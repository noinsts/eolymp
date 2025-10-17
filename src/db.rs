use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

const DATABASE_URL: &str = "eolymp.db";

table! {
    problems (id) {
        id -> Integer,
        problem_id -> Integer,
        name -> Text,
        url -> Text,
    }
}

#[derive(Queryable, Insertable, Clone, Debug)]
#[diesel(table_name = problems)]
pub struct Problem {
    pub id: i32,
    pub problem_id: i32,
    pub url: String,
    pub name: String,
}

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(table_name = problems)]
pub struct NewProblem {
    pub problem_id: i32,
    pub name: String,
    pub url: String,
}

pub struct Database {
    connection: SqliteConnection,
}

impl Database {
    pub fn new() -> Result<Self, ConnectionError> {
        let mut connection = SqliteConnection::establish(DATABASE_URL)?;

        diesel::sql_query(
            "CREATE TABLE IF NOT EXISTS problems (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                problem_id INTEGER NOT NULL UNIQUE,
                name TEXT NOT NULL,
                url TEXT NOT NULL
            )"
        )
            .execute(&mut connection)
            .expect("Помилка при створенні таблиці");

        Ok(Database { connection })
    }

    pub fn save_problem(&mut self, problem_id: i32, name: String, url: String) -> Result<(), diesel::result::Error> {
        let new_problem = NewProblem {
            problem_id,
            name,
            url: url.to_string(),
        };

        diesel::insert_into(problems::table)
            .values(&new_problem)
            .on_conflict(problems::problem_id)
            .do_update()
            .set((&new_problem))
            .execute(&mut self.connection)?;

        Ok(())
    }

    pub fn get_all_problems(&mut self) -> Result<Vec<Problem>, diesel::result::Error> {
        problems::table
            .load::<Problem>(&mut self.connection)
    }
}