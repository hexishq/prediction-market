const API_URL: &str = "http://localhost:3000"; // Em produção: "https://api.meusite.com"

#[tokio::main]
async fn main() -> Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;
    println!(
        "{}",
        style("🎲 SOLANA BETTING CLI (CLIENT) 🎲").magenta().bold()
    );

    // Setup
    let wallet = load_wallet()?;
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".to_string());
    let http_client = Client::new();

    println!("💳 Conectado como: {}", style(wallet.pubkey()).cyan());

    loop {
        let choices = &[
            "🔍 Listar Apostas",
            "➕ Criar Nova Aposta",
            "💰 Apostar",
            "❌ Sair",
        ];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Menu Principal")
            .default(0)
            .items(choices)
            .interact()?;

        match selection {
            0 => list_bets(&http_client).await?,
            1 => create_bet(&http_client, &wallet).await?,
            2 => place_bet(&http_client, &rpc_client, &wallet).await?,
            3 => break,
            _ => {}
        }
        println!();
    }
    Ok(())
}

// --- HTTP REQUESTS ---

async fn list_bets(client: &Client) -> Result<Vec<Bet>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Buscando dados do servidor...");

    let res = client.get(format!("{}/bets", API_URL)).send().await?;

    if !res.status().is_success() {
        spinner.finish_with_message(style("Erro ao conectar na API").red().to_string());
        return Ok(vec![]);
    }

    let bets: Vec<Bet> = res.json().await?;
    spinner.finish_and_clear();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec!["Título", "Valor (SOL)", "Pubkey"]);

    for bet in &bets {
        table.add_row(vec![
            &bet.title,
            &format!("{:.2}", bet.amount),
            &bet.pubkey[0..8],
        ]);
    }
    println!("{table}");

    Ok(bets)
}

async fn create_bet(client: &Client, wallet: &Keypair) -> Result<()> {
    // 1. Coleta inputs
    let title: String = Input::new().with_prompt("Título").interact_text()?;
    let amount: f64 = Input::new().with_prompt("Valor (SOL)").interact_text()?;

    // 2. Gera conta de aposta (Lógica OnChain simplificada)
    let bet_account = Keypair::new();
    println!("Gerando endereço da aposta: {}", bet_account.pubkey());

    // Aqui entraria a chamada RPC para criar a conta na Solana...
    // simulando delay
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // 3. Envia metadados para o Servidor
    let payload = CreateBetRequest {
        title,
        amount,
        creator: wallet.pubkey().to_string(),
        pubkey: bet_account.pubkey().to_string(),
    };

    let res = client
        .post(format!("{}/bets", API_URL))
        .json(&payload)
        .send()
        .await?;

    if res.status().is_success() {
        println!("{}", style("✔ Aposta registrada no servidor!").green());
    } else {
        println!("{}", style("✖ Erro ao registrar no servidor.").red());
    }

    Ok(())
}

async fn place_bet(http_client: &Client, rpc_client: &RpcClient, wallet: &Keypair) -> Result<()> {
    // Reusa a função de listar para pegar os dados
    let bets = list_bets(http_client).await?;
    if bets.is_empty() {
        return Ok(());
    }

    let items: Vec<String> = bets
        .iter()
        .map(|b| format!("{} - {} SOL", b.title, b.amount))
        .collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Escolha a aposta")
        .items(&items)
        .interact()?;

    let target_bet = &bets[selection];

    // Lógica Solana de Pagamento
    let to_pubkey = Pubkey::from_str(&target_bet.pubkey)?;
    let lamports = (target_bet.amount * LAMPORTS_PER_SOL as f64) as u64;

    let instruction = system_instruction::transfer(&wallet.pubkey(), &to_pubkey, lamports);
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&wallet.pubkey()),
        &[wallet],
        recent_blockhash,
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Enviando transação...");

    match rpc_client.send_and_confirm_transaction(&tx) {
        Ok(sig) => {
            spinner.finish_with_message(format!("{} Pago! Hash: {}", style("✔").green(), sig))
        }
        Err(e) => spinner.finish_with_message(format!("{} Erro: {}", style("✖").red(), e)),
    }

    Ok(())
}

fn load_wallet() -> Result<Keypair> {
    let home = dirs::home_dir().unwrap();
    read_keypair_file(home.join(".config/solana/id.json")).map_err(|e| anyhow::anyhow!(e))
}
