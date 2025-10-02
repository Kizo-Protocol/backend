use crate::error::AppError;
use crate::models::{BetCount, User, UserBetInfo, UserBetMarketInfo, UserWithBets};
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_or_create_user(&self, address: &str) -> Result<User, AppError> {
        let existing = sqlx::query_as::<_, User>(
            r#"
            SELECT id, address, username, "avatarUrl", "createdAt", "updatedAt"
            FROM users
            WHERE address = $1
            "#,
        )
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(user) = existing {
            return Ok(user);
        }

        let user_id = Uuid::new_v4().to_string();
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, address, username, "avatarUrl", "createdAt", "updatedAt")
            VALUES ($1, $2, NULL, NULL, NOW(), NOW())
            RETURNING id, address, username, "avatarUrl", "createdAt", "updatedAt"
            "#,
        )
        .bind(&user_id)
        .bind(address)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, address, username, "avatarUrl", "createdAt", "updatedAt"
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn update_profile(
        &self,
        user_id: &str,
        username: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<User, AppError> {
        if let Some(ref uname) = username {
            if uname.len() < 3 || uname.len() > 30 {
                return Err(AppError::BadRequest(
                    "Username must be between 3 and 30 characters".to_string(),
                ));
            }

            if !uname
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Err(AppError::BadRequest(
                    "Username can only contain letters, numbers, hyphens, and underscores"
                        .to_string(),
                ));
            }
        }

        let mut query = String::from(r#"UPDATE users SET "updatedAt" = NOW()"#);
        let mut param_count = 1;
        let mut params: Vec<String> = Vec::new();

        if username.is_some() {
            query.push_str(&format!(r#", username = ${}"#, param_count));
            param_count += 1;
            params.push(username.clone().unwrap_or_default());
        }

        if avatar_url.is_some() {
            query.push_str(&format!(r#", "avatarUrl" = ${}"#, param_count));
            param_count += 1;
            params.push(avatar_url.clone().unwrap_or_default());
        }

        query.push_str(&format!(r#" WHERE id = ${} RETURNING id, address, username, "avatarUrl", "createdAt", "updatedAt""#, param_count));
        params.push(user_id.to_string());

        let mut query_builder = sqlx::query_as::<_, User>(&query);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let user = query_builder.fetch_one(&self.pool).await?;

        Ok(user)
    }

    pub async fn get_user_with_bets(&self, user_id: &str) -> Result<UserWithBets, AppError> {
        let user = self
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let bets = sqlx::query_as::<
            _,
            (
                String,
                String,
                bool,
                String,
                String,
                Option<String>,
                String,
                chrono::NaiveDateTime,
                String,
                String,
                String,
                String,
                Option<String>,
            ),
        >(
            r#"
            SELECT
                be.id,
                COALESCE(me.id, '') as market_id,
                COALESCE(be.position, false) as position,
                COALESCE(be.amount::text, '0') as amount,
                COALESCE(be.odds::text, '0') as odds,
                be.payout::text as payout,
                COALESCE(be.status, 'active') as status,
                be."createdAt",
                COALESCE(me.id, '') as market_extended_id,
                COALESCE(me."adjTicker", '') as adj_ticker,
                COALESCE(m.question, '') as question,
                COALESCE(me.status, 'active') as market_status,
                me."imageUrl" as image_url
            FROM bets_extended be
            LEFT JOIN bets b ON be."blockchainBetId" = b.bet_id
            LEFT JOIN markets m ON b.market_id = m.market_id
            LEFT JOIN markets_extended me ON me.id = be."marketId"
            WHERE be."userId" = $1
            ORDER BY be."createdAt" DESC
            LIMIT 100
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let user_bets: Vec<UserBetInfo> = bets
            .into_iter()
            .map(
                |(
                    id,
                    market_id,
                    position,
                    amount_str,
                    odds,
                    payout,
                    status,
                    created_at,
                    _,
                    adj_ticker,
                    question,
                    market_status,
                    image_url,
                )| {
                    let amount_apt = amount_str
                        .parse::<f64>()
                        .map(|v| (v / 100_000_000.0).to_string())
                        .unwrap_or_else(|_| "0".to_string());

                    UserBetInfo {
                        id,
                        market_id: market_id.clone(),
                        position,
                        amount: amount_apt,
                        odds,
                        payout: payout.as_ref().map(|p| {
                            if let Ok(val) = p.parse::<f64>() {
                                (val / 100_000_000.0).to_string()
                            } else {
                                "0".to_string()
                            }
                        }),
                        status,
                        created_at,
                        market: UserBetMarketInfo {
                            id: market_id,
                            adj_ticker,
                            question,
                            status: market_status,
                            image_url,
                        },
                    }
                },
            )
            .collect();

        let bet_count = user_bets.len() as i64;

        Ok(UserWithBets {
            user,
            bets: user_bets,
            count: BetCount { bets: bet_count },
        })
    }
}
