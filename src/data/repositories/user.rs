use crate::data::models::NewUser;
use crate::data::models::User;
use crate::schema::users;
use bcrypt::verify;
use bcrypt::{DEFAULT_COST, hash};
use diesel::prelude::*;

pub struct UserRepository;

impl UserRepository {
    pub fn find_by_email(
        conn: &mut SqliteConnection,
        email: &str,
    ) -> Result<Option<User>, diesel::result::Error> {
        users::table
            .filter(users::email.eq(email))
            .first::<User>(conn)
            .optional()
    }

    pub fn verify_password(
        stored_hash: &str,
        input_password: &str,
    ) -> Result<bool, bcrypt::BcryptError> {
        verify(input_password, stored_hash)
    }

    pub fn create_user(
        conn: &mut SqliteConnection,
        email: &str,
        password: &str,
    ) -> Result<User, diesel::result::Error> {
        let hashed_password =
            hash(password, DEFAULT_COST).map_err(|_| diesel::result::Error::RollbackTransaction)?;

        diesel::insert_into(users::table)
            .values(&NewUser {
                email,
                password: &hashed_password,
            })
            .execute(conn)?;

        users::table
            .filter(users::email.eq(email))
            .first::<User>(conn)
    }

    pub fn email_exists(
        conn: &mut SqliteConnection,
        email: &str,
    ) -> Result<bool, diesel::result::Error> {
        use diesel::dsl::exists;
        use diesel::select;

        select(exists(users::table.filter(users::email.eq(email)))).get_result(conn)
    }
}