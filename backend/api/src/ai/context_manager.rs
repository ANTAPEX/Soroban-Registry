use crate::ai::models::{ChatMessage, ChatSession};
use shared::models::Contract;
use sqlx::{postgres::PgPool, types::Uuid};
use std::sync::Arc;

/// Manages conversation context and chat history for AI sessions
#[derive(Clone)]
pub struct ContextManager {
    db: PgPool,
}

impl ContextManager {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Create a new chat session
    pub async fn create_session(
        &self,
        user_id: Option<Uuid>,
        contract_id: Option<Uuid>,
        context_type: &str,
    ) -> sqlx::Result<ChatSession> {
        let session = sqlx::query_as!(
            ChatSession,
            r#"
            INSERT INTO ai_chat_sessions (user_id, contract_id, context_type)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, contract_id, session_title, context_type, 
                      created_at, updated_at, message_count, is_active
            "#,
            user_id,
            contract_id,
            context_type
        )
        .fetch_one(&self.db)
        .await?;

        Ok(session)
    }

    /// Get session with messages
    pub async fn get_session_with_messages(
        &self,
        session_id: Uuid,
    ) -> sqlx::Result<(ChatSession, Vec<ChatMessage>)> {
        let session = sqlx::query_as!(
            ChatSession,
            "SELECT id, user_id, contract_id, session_title, context_type, created_at, updated_at, message_count, is_active FROM ai_chat_sessions WHERE id = $1",
            session_id
        )
        .fetch_one(&self.db)
        .await?;

        let messages = sqlx::query_as!(
            ChatMessage,
            r#"
            SELECT id, session_id, role, content, contract_code_snippet, 
                   token_count, model_used, response_time_ms, created_at, metadata
            FROM ai_chat_messages 
            WHERE session_id = $1 
            ORDER BY created_at ASC
            "#,
            session_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok((session, messages))
    }

    /// Add a message to session
    pub async fn add_message(
        &self,
        session_id: Uuid,
        role: &str,
        content: &str,
        contract_code_snippet: Option<&str>,
        token_count: Option<i32>,
        model_used: Option<&str>,
        response_time_ms: Option<i32>,
        metadata: Option<Value>,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO ai_chat_messages (
                session_id, role, content, contract_code_snippet, 
                token_count, model_used, response_time_ms, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            session_id,
            role,
            content,
            contract_code_snippet,
            token_count,
            model_used,
            response_time_ms,
            metadata
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get recent sessions for a user
    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
        limit: i32,
    ) -> sqlx::Result<Vec<ChatSession>> {
        let sessions = sqlx::query_as!(
            ChatSession,
            r#"
            SELECT id, user_id, contract_id, session_title, context_type, 
                   created_at, updated_at, message_count, is_active
            FROM ai_chat_sessions 
            WHERE user_id = $1 
            ORDER BY updated_at DESC 
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        Ok(sessions)
    }

    /// Get contract context from database
    pub async fn get_contract_context(
        &self,
        contract_id: Uuid,
    ) -> sqlx::Result<Option<ContractContext>> {
        let result = sqlx::query_as!(
            ContractContext,
            r#"
            SELECT 
                c.id::text as contract_id,
                c.name as contract_name,
                c.slug,
                c.description,
                c.category,
                COALESCE(
                    json_agg(t.name) FILTER (WHERE t.id IS NOT NULL),
                    '[]'::jsonb
                ) as tags,
                c.network as network
            FROM contracts c
            LEFT JOIN contract_tags ct ON c.id = ct.contract_id
            LEFT JOIN tags t ON ct.tag_id = t.id
            WHERE c.id = $1
            GROUP BY c.id, c.name, c.slug, c.description, c.category, c.network
            "#,
            contract_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(result)
    }

    /// Update session title based on first message
    pub async fn update_session_title(
        &self,
        session_id: Uuid,
        first_message: &str,
    ) -> sqlx::Result<()> {
        let title = if first_message.len() > 50 {
            format!("{}...", &first_message[..47])
        } else {
            first_message.to_string()
        };

        sqlx::query!(
            "UPDATE ai_chat_sessions SET session_title = $1 WHERE id = $2",
            title,
            session_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
