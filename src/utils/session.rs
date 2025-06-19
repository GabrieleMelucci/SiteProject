use tower_sessions::Session;
use crate::data::models::LoginError;

pub async fn set_user_session(
    session: &Session,
    user_id: i32,
    email: &str,
) -> Result<(), LoginError> {
    session.insert("logged_in", true).await?;
    session.insert("user_id", user_id).await?;
    session.insert("user_email", email).await?;
    Ok(())
}

pub async fn is_logged_in(session: &Session) -> bool {
    session.get::<i32>("user_id").await.unwrap_or(None).is_some()
}

pub async fn get_current_user_id(session: &Session) -> Option<i32> {
    if !is_logged_in(session).await {
        return None;
    }

    match session.get::<i32>("user_id").await {
        Ok(Some(user_id)) => Some(user_id),
        Ok(None) => {
            log::warn!("Session has logged_in=true but no user_id");
            None
        },
        Err(e) => {
            log::error!("Failed to get user_id from session: {}", e);
            None
        }
    }
}