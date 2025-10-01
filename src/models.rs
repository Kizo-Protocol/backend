use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize, Serializer};
use sqlx::FromRow;
use sqlx::types::BigDecimal;
use utoipa::ToSchema;


fn serialize_naive_datetime<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = date.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    serializer.serialize_str(&s)
}



#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub address: String,
    pub username: Option<String>,
    #[serde(rename = "avatarUrl")]
    #[sqlx(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    #[serde(rename = "createdAt")]
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    #[sqlx(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WalletConnectRequest {
    pub address: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletConnectResponse {
    pub message: String,
    pub data: WalletConnectData,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletConnectData {
    pub user: User,
    pub token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateProfileResponse {
    pub message: String,
    pub data: UpdateProfileData,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateProfileData {
    pub user: User,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserWithBets {
    #[serde(flatten)]
    pub user: User,
    pub bets: Vec<UserBetInfo>,
    #[serde(rename = "_count")]
    pub count: BetCount,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserBetInfo {
    pub id: String,
    pub market_id: String,
    pub position: bool,
    pub amount: String,
    pub odds: String,
    pub payout: Option<String>,
    pub status: String,
    #[serde(serialize_with = "serialize_naive_datetime")]
    pub created_at: NaiveDateTime,
    pub market: UserBetMarketInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserBetMarketInfo {
    pub id: String,
    pub adj_ticker: String,
    pub question: String,
    pub status: String,
    pub image_url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub address: String,
    pub email: Option<String>,
    pub username: Option<String>,
}



#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Market {
    pub market_id: i64,
    pub question: String,
    pub end_time: i64,
    pub yield_protocol_addr: String,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub inserted_at: NaiveDateTime,
    pub resolved: Option<bool>,
    pub outcome: Option<bool>,
    pub total_yield_earned: Option<i64>,
    pub resolution_transaction_version: Option<i64>,
}


#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketExtended {
    pub id: String,
    #[sqlx(rename = "blockchainMarketId")]
    pub blockchain_market_id: Option<i64>,
    #[sqlx(rename = "marketId")]
    pub market_id: Option<String>,
    #[sqlx(rename = "adjTicker")]
    pub adj_ticker: Option<String>,
    pub platform: String,
    pub question: Option<String>,
    pub description: Option<String>,
    pub rules: Option<String>,
    pub status: String,
    pub probability: i32,
    pub volume: BigDecimal,
    #[sqlx(rename = "openInterest")]
    pub open_interest: BigDecimal,
    #[sqlx(rename = "endDate")]
    pub end_date: NaiveDateTime,
    #[sqlx(rename = "resolutionDate")]
    pub resolution_date: Option<NaiveDateTime>,
    pub result: Option<bool>,
    pub link: Option<String>,
    #[sqlx(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[sqlx(rename = "totalPoolSize")]
    pub total_pool_size: BigDecimal,
    #[sqlx(rename = "yesPoolSize")]
    pub yes_pool_size: BigDecimal,
    #[sqlx(rename = "noPoolSize")]
    pub no_pool_size: BigDecimal,
    #[sqlx(rename = "countYes")]
    pub count_yes: i32,
    #[sqlx(rename = "countNo")]
    pub count_no: i32,
    #[sqlx(rename = "currentYield")]
    pub current_yield: BigDecimal,
    #[sqlx(rename = "totalYieldEarned")]
    pub total_yield_earned: BigDecimal,
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MarketResponse {
    pub id: String,
    pub blockchain_market_id: Option<i64>,
    pub market_id: Option<String>,
    pub adj_ticker: Option<String>,
    pub platform: String,
    pub question: Option<String>,
    pub description: Option<String>,
    pub rules: Option<String>,
    pub status: String,
    pub probability: i32,
    pub volume: String,
    pub open_interest: String,
    pub end_date: NaiveDateTime,
    pub resolution_date: Option<NaiveDateTime>,
    pub result: Option<bool>,
    pub link: Option<String>,
    pub image_url: Option<String>,
    pub total_pool_size: String,
    pub yes_pool_size: String,
    pub no_pool_size: String,
    pub count_yes: i32,
    pub count_no: i32,
    pub current_yield: String,
    pub total_yield_earned: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_yield: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_yield_until_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_remaining: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_protocol_apy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_protocol_name: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bets: Option<Vec<BetResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _count: Option<BetCount>,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct BetCount {
    pub bets: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct BetResponse {
    pub id: String,
    pub amount: String,
    pub position: Option<bool>,
    pub odds: Option<String>,
    pub status: String,
    pub created_at: NaiveDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UserInfo {
    pub id: String,
    pub address: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketWithStats {
    #[serde(flatten)]
    pub market: Market,
    pub total_bets: i64,
    pub total_volume: String,
    pub yes_volume: String,
    pub no_volume: String,
    pub yes_percentage: f64,
    pub no_percentage: f64,
}



#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Bet {
    pub bet_id: i64,
    pub market_id: i64,
    pub user_addr: String,
    pub position: bool,
    pub amount: i64,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub inserted_at: NaiveDateTime,
    pub claimed: Option<bool>,
    pub winning_amount: Option<i64>,
    pub yield_share: Option<i64>,
    pub claim_transaction_version: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BetWithMarket {
    pub bet_id: i64,
    pub market_id: i64,
    pub user_addr: String,
    pub position: bool,
    pub amount: i64,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub inserted_at: NaiveDateTime,
    pub claimed: Option<bool>,
    pub winning_amount: Option<i64>,
    pub yield_share: Option<i64>,
    pub claim_transaction_version: Option<i64>,
    pub market_question: String,
}



#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketResolution {
    pub market_id: i64,
    pub outcome: bool,
    pub total_yield_earned: i64,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub inserted_at: NaiveDateTime,
}



#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WinningsClaim {
    pub claim_id: i64,
    pub bet_id: i64,
    pub user_addr: String,
    pub winning_amount: i64,
    pub yield_share: i64,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub inserted_at: NaiveDateTime,
}



#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct YieldDeposit {
    pub deposit_id: i64,
    pub market_id: i64,
    pub amount: i64,
    pub protocol_addr: String,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub inserted_at: NaiveDateTime,
}



#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolFee {
    pub fee_id: i64,
    pub market_id: i64,
    pub fee_amount: i64,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub inserted_at: NaiveDateTime,
}



#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarketStats {
    pub market_id: i64,
    pub total_bets: i64,
    pub total_volume: String,
    pub yes_volume: String,
    pub no_volume: String,
    pub yes_percentage: f64,
    pub no_percentage: f64,
    pub unique_bettors: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformStats {
    pub total_markets: i64,
    pub active_markets: i64,
    pub resolved_markets: i64,
    pub total_bets: i64,
    pub total_volume: String,
    pub unique_users: i64,
    pub total_yield_earned: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserStats {
    pub user_addr: String,
    pub total_bets: i64,
    pub total_wagered: String,
    pub markets_participated: i64,
    pub wins: i64,
    pub losses: i64,
    pub pending: i64,
    pub total_winnings: String,
    pub total_yield_earned: String,
}



#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub last_indexed_version: i64,
    pub last_indexed_block_height: i64,
    pub event_type: String,
    pub last_sync_time: NaiveDateTime,
}



#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct MarketQueryParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub status: MarketStatus,
    #[serde(default)]
    pub sort_by: MarketSortBy,
    #[serde(default)]
    pub order: SortOrder,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MarketStatus {
    #[default]
    Active,
    Resolved,
    All,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MarketSortBy {
    #[default]
    EndTime,
    TransactionVersion,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}



#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Protocol {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub base_apy: BigDecimal,
    pub is_active: bool,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}



#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct YieldRecord {
    pub id: String,
    pub market_id: String,
    pub protocol_id: String,
    pub amount: BigDecimal,
    pub apy: BigDecimal,
    pub yield_amount: BigDecimal,
    pub period: NaiveDateTime,
    pub created_at: NaiveDateTime,
}



#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FeeRecord {
    pub id: String,
    pub market_id: Option<String>,
    pub fee_type: String,
    pub amount: BigDecimal,
    pub source: String,
    pub created_at: NaiveDateTime,
}



#[derive(Debug, Serialize, Deserialize)]
pub struct ChartDataPoint {
    pub time: i64,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketChartData {
    pub yes_probability: Vec<ChartDataPoint>,
    pub no_probability: Vec<ChartDataPoint>,
    pub yes_volume: Vec<ChartDataPoint>,
    pub no_volume: Vec<ChartDataPoint>,
    pub total_volume: Vec<ChartDataPoint>,
    pub yes_odds: Vec<ChartDataPoint>,
    pub no_odds: Vec<ChartDataPoint>,
    pub bet_count: Vec<ChartDataPoint>,
}

#[derive(Debug, Deserialize)]
pub struct ChartQueryParams {
    #[serde(default = "default_interval")]
    pub interval: String,
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[serde(default = "default_series")]
    pub series: String,
}

fn default_interval() -> String {
    "1h".to_string()
}

fn default_series() -> String {
    "probability,volume,odds".to_string()
}
