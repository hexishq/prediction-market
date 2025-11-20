use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use dotenv::dotenv;
use firestore::FirestoreDb;
use std::env;
use std::sync::Arc;
use tracing::{error, info};
use types::{Bet, CreateBetRequest};

struct AppState {
    db: FirestoreDb,
    collection: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let project_id = env::var("FIREBASE_PROJECT_ID").expect("FIREBASE_PROJECT_ID not found .env");
    let db = FirestoreDb::new(&project_id).await?;

    let shared_state = Arc::new(AppState {
        db,
        collection: "bets".to_string(),
    });

    // Rotas
    let app = Router::new()
        .route("/bets", get(list_bets).post(create_bet))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("🚀 Servidor rodando em http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

// GET /bets
async fn list_bets(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Bet>>, StatusCode> {
    let bets: Vec<Bet> = state
        .db
        .fluent()
        .select()
        .from(state.collection.as_str())
        .obj()
        .query()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(bets))
}

// POST /bets
async fn create_bet(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateBetRequest>,
) -> Result<Json<Bet>, StatusCode> {
    // NOTA DE SEGURANÇA: Em produção, você deve verificar a assinatura da transação Solana aqui
    // para garantir que o usuário realmente depositou o dinheiro antes de salvar no banco.

    let new_bet = Bet {
        id: String::new(),
        title: payload.title,
        amount: payload.amount,
        creator: payload.creator,
        pubkey: payload.pubkey.clone(),
        status: "OPEN".to_string(),
    };

    state
        .db
        .fluent()
        .insert()
        .into(&state.collection)
        .document_id(&payload.pubkey)
        .object(&new_bet)
        .execute::<()>()
        .await
        .map_err(|e| {
            error!("Failed on save: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(new_bet))
}
