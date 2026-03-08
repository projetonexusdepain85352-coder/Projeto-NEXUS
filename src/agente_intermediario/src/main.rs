use sha2::{Sha256, Digest};
use uuid::Uuid;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};

// =============================================================================
// CONFIGURAÇÃO
// =============================================================================

/// Configuração global do agente.
struct Config {
    max_paginas: usize,
    max_profundidade: usize,
    delay_ms: u64,
    // Limiares de qualidade — configuráveis sem recompilar
    qualidade_min_bytes: usize,
    qualidade_max_duplicadas_pct: f64,
    qualidade_max_curtas_pct: f64,
    qualidade_min_pontuacao_pct: f64,
    // Limites de payload
    max_bytes_html: usize,
    max_bytes_pdf: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_paginas: 100,
            max_profundidade: 2,
            delay_ms: 600,
            qualidade_min_bytes: 800,
            qualidade_max_duplicadas_pct: 0.55,
            qualidade_max_curtas_pct: 0.80,
            qualidade_min_pontuacao_pct: 0.005,
            max_bytes_html: 10 * 1024 * 1024,  // 10 MB
            max_bytes_pdf: 50 * 1024 * 1024,   // 50 MB
        }
    }
}

/// Configuração por fonte — permite ajuste fino sem alterar o padrão global.
struct FonteConfig<'a> {
    url: &'a str,
    domain: &'a str,
    doc_type: &'a str,
    max_paginas: Option<usize>,  // Override do global se definido
}

impl<'a> FonteConfig<'a> {
    fn new(url: &'a str, domain: &'a str, doc_type: &'a str) -> Self {
        FonteConfig { url, domain, doc_type, max_paginas: None }
    }

    fn com_limite(mut self, max: usize) -> Self {
        self.max_paginas = Some(max);
        self
    }
}

// =============================================================================
// FILTROS DE URL
// =============================================================================

const EXTENSOES_BINARIAS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".ico",
    ".woff", ".woff2", ".ttf", ".eot", ".otf",
    ".zip", ".tar", ".gz", ".bz2", ".xz",
    ".doc", ".docx", ".xls", ".xlsx",
    ".mp4", ".mp3", ".avi", ".mov", ".webm",
    ".exe", ".dll", ".so", ".dylib",
    ".css", ".map",
];

const PADROES_VERSAO: &[&str] = &[
    "/v0.", "/v1.", "/v2.", "/v3.", "/v4.", "/v5.",
    "/v6.", "/v7.", "/v8.", "/v9.",
];

const PADROES_INDICE_URL: &[&str] = &[
    "/index.html", "/index.htm", "/toc.html", "/toc.htm",
    "/contents.html", "/genindex", "/modindex",
];

const ASSINATURAS_LIXO: &[&str] = &[
    "The Linux Kernel\nQuick search\nDevelopment process",
    "Documentation\nYour account\n",
];

fn url_e_binaria(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    let caminho = url_lower.split('?').next().unwrap_or(&url_lower);
    EXTENSOES_BINARIAS.iter().any(|ext| caminho.ends_with(ext))
}

fn url_e_pdf(url: &str) -> bool {
    if url.to_lowercase().contains(".pdf.pdf") {
        return false;
    }
    let url_lower = url.to_lowercase();
    let caminho = url_lower.split('?').next().unwrap_or(&url_lower);
    caminho.ends_with(".pdf")
}

fn url_tem_versao(url: &str) -> bool {
    PADROES_VERSAO.iter().any(|p| url.contains(p))
}

fn url_parece_indice(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    let caminho = url_lower.split('?').next().unwrap_or(&url_lower);
    let caminho = caminho.split('#').next().unwrap_or(caminho);
    PADROES_INDICE_URL.iter().any(|p| caminho.ends_with(p))
}

/// Normaliza URL para evitar duplicatas lógicas na fila BFS.
/// Remove: fragmento #, parâmetros utm_*, trailing slash redundante.
fn canonicalizar_url(url: &str) -> String {
    // Remove fragmento
    let sem_fragmento = url.split('#').next().unwrap_or(url);

    // Faz parse para manipular query string
    if let Ok(mut parsed) = url::Url::parse(sem_fragmento) {
        // Remove parâmetros de rastreamento
        let params_filtrar = ["utm_source", "utm_medium", "utm_campaign",
                              "utm_term", "utm_content", "ref", "source"];
        let query_limpa: Vec<(String, String)> = parsed.query_pairs()
            .filter(|(k, _)| !params_filtrar.contains(&k.as_ref()))
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        if query_limpa.is_empty() {
            parsed.set_query(None);
        } else {
            let nova_query = query_limpa.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            parsed.set_query(Some(&nova_query));
        }

        parsed.to_string()
    } else {
        sem_fragmento.to_string()
    }
}

// =============================================================================
// DOWNLOAD COM RETRY
// =============================================================================

/// Resultado de download com informação sobre limite de tamanho excedido.

/// Baixa conteúdo HTML com retry exponencial (até 3 tentativas).
/// Respeita Retry-After quando presente. Rejeita respostas > max_bytes.
fn baixar_conteudo(url: &str, max_bytes: usize) -> Result<String, Box<dyn std::error::Error>> {
    let cliente = reqwest::blocking::Client::builder()
        .user_agent("NEXUS-Agent/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut tentativa = 0;
    let max_tentativas = 3;

    loop {
        tentativa += 1;
        match cliente.get(url).send() {
            Ok(resp) => {
                let status = resp.status();

                // Rate limit — aguarda se servidor indicar
                if status.as_u16() == 429 {
                    let aguardar = resp.headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(30);
                    eprintln!("  [RATE LIMIT] {} — aguardando {}s (tentativa {}/{})",
                        url, aguardar, tentativa, max_tentativas);
                    std::thread::sleep(std::time::Duration::from_secs(aguardar));
                    if tentativa >= max_tentativas { break; }
                    continue;
                }

                // Erros de servidor — retry com backoff
                if status.is_server_error() && tentativa < max_tentativas {
                    let delay = 2u64.pow(tentativa as u32);
                    eprintln!("  [ERRO HTTP {}] {} — retry em {}s", status, url, delay);
                    std::thread::sleep(std::time::Duration::from_secs(delay));
                    continue;
                }

                let conteudo = resp.text()?;

                if conteudo.len() > max_bytes {
                    return Err(format!(
                        "Payload muito grande: {} bytes (limite {})",
                        conteudo.len(), max_bytes
                    ).into());
                }

                return Ok(conteudo);
            }
            Err(e) => {
                if tentativa < max_tentativas {
                    let delay = 2u64.pow(tentativa as u32);
                    eprintln!("  [ERRO REDE] {} — retry em {}s: {}", url, delay, e);
                    std::thread::sleep(std::time::Duration::from_secs(delay));
                } else {
                    return Err(e.into());
                }
            }
        }
    }

    Err(format!("Falha após {} tentativas: {}", max_tentativas, url).into())
}

/// Baixa bytes binários (PDFs) com retry e limite de tamanho.
fn baixar_bytes(url: &str, max_bytes: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cliente = reqwest::blocking::Client::builder()
        .user_agent("NEXUS-Agent/0.1")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut tentativa = 0;
    let max_tentativas = 3;

    loop {
        tentativa += 1;
        match cliente.get(url).send() {
            Ok(resp) => {
                let status = resp.status();

                if status.as_u16() == 429 {
                    let aguardar = resp.headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(30);
                    std::thread::sleep(std::time::Duration::from_secs(aguardar));
                    if tentativa >= max_tentativas { break; }
                    continue;
                }

                if status.is_server_error() && tentativa < max_tentativas {
                    let delay = 2u64.pow(tentativa as u32);
                    std::thread::sleep(std::time::Duration::from_secs(delay));
                    continue;
                }

                let bytes = resp.bytes()?;

                if bytes.len() > max_bytes {
                    return Err(format!(
                        "PDF muito grande: {} bytes (limite {})",
                        bytes.len(), max_bytes
                    ).into());
                }

                return Ok(bytes.to_vec());
            }
            Err(e) => {
                if tentativa < max_tentativas {
                    let delay = 2u64.pow(tentativa as u32);
                    std::thread::sleep(std::time::Duration::from_secs(delay));
                } else {
                    return Err(e.into());
                }
            }
        }
    }

    Err(format!("Falha após {} tentativas: {}", max_tentativas, url).into())
}

// =============================================================================
// EXTRAÇÃO DE PDF
// =============================================================================

fn extrair_texto_pdf(bytes: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    // Tentativa 1: lopdf (nativo, sem processo externo)
    if let Ok(doc) = lopdf::Document::load_mem(bytes) {
        let mut texto = String::new();
        let paginas: Vec<u32> = doc.get_pages().keys().cloned().collect();
        for num_pagina in paginas {
            if let Ok(t) = doc.extract_text(&[num_pagina]) {
                let t = t.trim().to_string();
                if !t.is_empty() {
                    texto.push_str(&t);
                    texto.push('\n');
                }
            }
        }
        if texto.trim().len() >= 300 {
            return Ok(texto);
        }
    }

    // Tentativa 2: pdftotext externo com timeout de 30s
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(bytes)?;

    let output = std::process::Command::new("pdftotext")
        .arg(tmp.path())
        .arg("-")
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let texto = String::from_utf8_lossy(&out.stdout).to_string();
            if texto.trim().len() >= 300 {
                return Ok(texto);
            }
        }
        Ok(_) => {}
        Err(e) => {
            // pdftotext não instalado — avisa uma vez
            eprintln!("  [AVISO] pdftotext não disponível (instale poppler-utils): {}", e);
        }
    }

    Err("Nao foi possivel extrair texto do PDF".into())
}

// =============================================================================
// EXTRAÇÃO DE LINKS
// =============================================================================

fn extrair_links_pdf(html: &str, base_url: &str) -> Vec<String> {
    let documento = Html::parse_document(html);
    let seletor = Selector::parse("a[href]").unwrap();
    let base = match url::Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return vec![],
    };
    let dominio_base = base.host_str().unwrap_or("").to_string();
    let mut pdfs = Vec::new();

    for elemento in documento.select(&seletor) {
        if let Some(href) = elemento.value().attr("href") {
            if let Ok(url_absoluta) = base.join(href) {
                let url_str = url_absoluta.to_string();
                if url_e_pdf(&url_str) {
                    let dominio_pdf = url_absoluta.host_str().unwrap_or("");
                    if dominio_pdf != dominio_base {
                        continue;
                    }
                    let url_lower = url_str.to_lowercase();
                    let idiomas = ["arabic", "chinese", "korean", "japanese",
                                   "hebrew", "french", "spanish", "italian",
                                   "ukrainian", "vietnamese", "indonesian",
                                   "portuguese", "czech", "_ar", "_cn", "_jp",
                                   "_ko", "_fr", "_es", "_it", "_pt", "_de"];
                    if idiomas.iter().any(|i| url_lower.contains(i)) {
                        continue;
                    }
                    let canonical = canonicalizar_url(&url_str);
                    pdfs.push(canonical);
                }
            }
        }
    }
    pdfs
}

fn extrair_links(html: &str, base_url: &str) -> Vec<String> {
    let documento = Html::parse_document(html);
    let seletor = Selector::parse("a[href]").unwrap();
    let base = match url::Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return vec![],
    };
    let mut links = Vec::new();
    let partes_ignorar = [
        "/de/", "/fr/", "/es/", "/pt/", "/zh/", "/ja/",
        "/ko/", "/ru/", "/it/", "/pl/", "/nl/",
    ];

    for elemento in documento.select(&seletor) {
        if let Some(href) = elemento.value().attr("href") {
            if let Ok(url_absoluta) = base.join(href) {
                let url_str = url_absoluta.to_string();
                if url_str.starts_with(base_url) {
                    let ignorar_idioma = partes_ignorar.iter().any(|p| url_str.contains(p));
                    let ignorar_versao = url_tem_versao(&url_str);
                    let ignorar_binario = url_e_binaria(&url_str);
                    let ignorar_indice = url_parece_indice(&url_str);
                    let ignorar_pdf = url_e_pdf(&url_str);

                    if !ignorar_idioma && !ignorar_versao && !ignorar_binario
                        && !ignorar_indice && !ignorar_pdf {
                        let canonical = canonicalizar_url(&url_str);
                        if !canonical.is_empty() {
                            links.push(canonical);
                        }
                    }
                }
            }
        }
    }
    links
}

// =============================================================================
// EXTRAÇÃO DE TEXTO HTML
// =============================================================================

fn extrair_texto_limpo(html: &str) -> String {
    let documento = Html::parse_document(html);

    let seletores_principais = [
        "main article", "main", "article", "[role='main']",
        ".content", "#content", ".document", "#document", ".body",
        ".post-content", ".article-body", ".markdown-body", ".rst-content",
        ".devsite-article-body",
    ];

    let seletor_ignorar = Selector::parse(
        "nav, aside, header, footer, \
         .sidebar, .navigation, .nav, .toc, \
         .breadcrumb, .breadcrumbs, .menu, .navbar, \
         #sidebar, #navigation, #toc, #nav, \
         [role='navigation'], [role='banner'], [role='contentinfo'], \
         script, style, noscript"
    ).unwrap();

    for seletor_str in &seletores_principais {
        if let Ok(seletor) = Selector::parse(seletor_str) {
            if let Some(elemento_principal) = documento.select(&seletor).next() {
                let html_principal = elemento_principal.html();
                let doc_interno = Html::parse_fragment(&html_principal);

                let seletor_conteudo = Selector::parse(
                    "p, h1, h2, h3, h4, h5, h6, li, td, th, pre, code, blockquote, dt, dd"
                ).unwrap();

                let mut texto = String::new();
                let mut nos_nav: HashSet<String> = HashSet::new();

                for nav_elem in doc_interno.select(&seletor_ignorar) {
                    nos_nav.insert(nav_elem.html());
                }

                for elem in doc_interno.select(&seletor_conteudo) {
                    let html_elem = elem.html();
                    if nos_nav.iter().any(|n| n.contains(&html_elem)) {
                        continue;
                    }
                    let texto_elem = elem.text().collect::<Vec<_>>().join(" ");
                    let texto_elem = texto_elem.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !texto_elem.is_empty() && texto_elem.len() > 10 {
                        texto.push_str(&texto_elem);
                        texto.push('\n');
                    }
                }

                if texto.trim().len() >= 200 {
                    return texto;
                }
            }
        }
    }

    // Fallback genérico
    let seletor_conteudo = Selector::parse(
        "p, h1, h2, h3, h4, h5, h6, pre, code, blockquote, article, td, th, dt, dd"
    ).unwrap();

    let seletor_nav = Selector::parse(
        "nav *, aside *, header *, footer *, \
         .sidebar *, .navigation *, [role='navigation'] *"
    ).unwrap();

    let nos_nav: HashSet<String> = documento
        .select(&seletor_nav)
        .map(|e| e.html())
        .collect();

    let mut texto = String::new();
    for elemento in documento.select(&seletor_conteudo) {
        let html_elem = elemento.html();
        if nos_nav.contains(&html_elem) {
            continue;
        }
        let texto_elem = elemento.text().collect::<Vec<_>>().join(" ");
        let texto_elem = texto_elem.split_whitespace().collect::<Vec<_>>().join(" ");
        if !texto_elem.is_empty() && texto_elem.len() > 10 {
            texto.push_str(&texto_elem);
            texto.push('\n');
        }
    }

    // Fallback total
    if texto.trim().len() < 100 {
        let doc2 = Html::parse_document(html);
        texto = doc2.root_element().text().collect::<Vec<_>>().join(" ");
        texto = texto.split_whitespace().collect::<Vec<_>>().join(" ");
    }

    texto
}

// =============================================================================
// ANÁLISE DE QUALIDADE (limiares via Config)
// =============================================================================

fn analisar_qualidade(texto: &str, config: &Config) -> (bool, &'static str) {
    let linhas: Vec<&str> = texto.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if linhas.is_empty() {
        return (false, "sem conteudo");
    }

    let total = linhas.len();

    for assinatura in ASSINATURAS_LIXO {
        if texto.contains(assinatura) {
            return (false, "assinatura de pagina de navegacao");
        }
    }

    let mut contagem: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for linha in &linhas {
        *contagem.entry(linha).or_insert(0) += 1;
    }
    let duplicadas: usize = contagem.values().filter(|&&c| c > 1).map(|&c| c - 1).sum();
    let proporcao_duplicadas = duplicadas as f64 / total as f64;
    if proporcao_duplicadas > config.qualidade_max_duplicadas_pct {
        return (false, "muitas linhas duplicadas (indice/navegacao)");
    }

    let linhas_curtas = linhas.iter().filter(|l| l.len() < 25).count();
    let proporcao_curtas = linhas_curtas as f64 / total as f64;
    if proporcao_curtas > config.qualidade_max_curtas_pct && total > 20 {
        return (false, "muitas linhas curtas (lista de links/navegacao)");
    }

    let texto_sem_quebras = texto.replace('\n', " ");
    let chars_total = texto_sem_quebras.len();
    if chars_total > 500 {
        let pontuacao = texto_sem_quebras.chars()
            .filter(|&c| c == '.' || c == ',' || c == ';' || c == ':')
            .count();
        let proporcao_pontuacao = pontuacao as f64 / chars_total as f64;
        if proporcao_pontuacao < config.qualidade_min_pontuacao_pct && total > 30 {
            return (false, "ausencia de pontuacao (provavelmente lista de titulos)");
        }
    }

    if texto.trim().len() < config.qualidade_min_bytes {
        return (false, "conteudo muito curto");
    }

    (true, "")
}

// =============================================================================
// HASH E BANCO DE DADOS
// =============================================================================

fn calcular_hash(conteudo: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(conteudo.as_bytes());
    hex::encode(hasher.finalize())
}

fn hash_armazenado(client: &mut postgres::Client, url: &str) -> Option<String> {
    match client.query_opt(
        "SELECT content_hash FROM documents WHERE source = $1 ORDER BY collected_at DESC LIMIT 1",
        &[&url],
    ) {
        Ok(Some(row)) => Some(row.get(0)),
        _ => None,
    }
}

fn e_erro_duplicata(e: &Box<dyn std::error::Error>) -> bool {
    let msg = format!("{:?}", e);
    msg.contains("duplicate key") || msg.contains("E23505")
}

fn atualizar_documento(
    client: &mut postgres::Client,
    url: &str,
    conteudo: &str,
    hash: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let tamanho = conteudo.len() as i32;
    client.execute(
        "UPDATE documents SET content = $1, content_hash = $2, content_length = $3, collected_at = NOW()
         WHERE source = $4",
        &[&conteudo, &hash, &tamanho, &url],
    )?;
    println!("  [ATUALIZADO] {} ({} bytes)", url, conteudo.len());
    Ok(())
}

fn inserir_documento(
    client: &mut postgres::Client,
    url: &str,
    domain: &str,
    doc_type: &str,
    conteudo: &str,
    hash: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = Uuid::new_v4();
    let tamanho = conteudo.len() as i32;

    client.execute(
        "INSERT INTO documents (id, content, source, domain, doc_type, content_hash, content_length, inserted_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'agent')",
        &[&id, &conteudo, &url, &domain, &doc_type, &hash, &tamanho],
    )?;

    let val_id = Uuid::new_v4();
    client.execute(
        "INSERT INTO validation (id, document_id, status, decided_by)
         VALUES ($1, $2, 'pending', 'agent')",
        &[&val_id, &id],
    )?;

    Ok(())
}

// =============================================================================
// PROCESSAMENTO DE PÁGINAS E PDFs
// =============================================================================

fn processar_pdf(
    client: &mut postgres::Client,
    url: &str,
    domain: &str,
    inseridos: &mut usize,
    ignorados: &mut usize,
    erros: &mut usize,
    max_paginas: usize,
    config: &Config,
) {
    if *inseridos >= max_paginas {
        return;
    }

    if hash_armazenado(client, url).is_some() {
        *ignorados += 1;
        return;
    }

    println!("  [PDF] Baixando {}", url);

    match baixar_bytes(url, config.max_bytes_pdf) {
        Ok(bytes) => {
            match extrair_texto_pdf(&bytes) {
                Ok(texto) => {
                    let texto = texto.trim().to_string();
                    if texto.len() < 300 {
                        println!("  [PDF FILTRADO] {} — conteudo muito curto", url);
                        return;
                    }
                    let hash = calcular_hash(&texto);
                    match inserir_documento(client, url, domain, "pdf", &texto, &hash) {
                        Ok(_) => {
                            *inseridos += 1;
                            println!("  [PDF OK] ({}/{}) {} ({} bytes)", inseridos, max_paginas, url, texto.len());
                        }
                        Err(e) => {
                            if e_erro_duplicata(&e) {
                                *ignorados += 1;
                            } else {
                                *erros += 1;
                                eprintln!("  [PDF ERRO DB] {}: {:?}", url, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    *erros += 1;
                    eprintln!("  [PDF ERRO EXTRACAO] {}: {}", url, e);
                }
            }
        }
        Err(e) => {
            *erros += 1;
            eprintln!("  [PDF ERRO HTTP] {}: {}", url, e);
        }
    }
}

fn processar_pagina(
    client: &mut postgres::Client,
    url: &str,
    domain: &str,
    doc_type: &str,
    html: &str,
    inseridos: &mut usize,
    ignorados: &mut usize,
    filtrados: &mut usize,
    erros: &mut usize,
    max_paginas: usize,
    config: &Config,
) {
    let pdfs = extrair_links_pdf(html, url);
    for pdf_url in pdfs {
        if *inseridos >= max_paginas { break; }
        processar_pdf(client, &pdf_url, domain, inseridos, ignorados, erros, max_paginas, config);
        std::thread::sleep(std::time::Duration::from_millis(config.delay_ms));
    }

    // Detecta arquivos plain text pela URL — pula extrator HTML
    // que colapsaria todas as quebras de linha via split_whitespace
    let extensoes_plain: &[&str] = &[".txt", ".md", ".rst", ".rst.txt", ".json"];
    let url_lower = url.to_lowercase();
    let e_plain_text = extensoes_plain.iter().any(|ext| url_lower.ends_with(ext));
    let texto = if e_plain_text {
        html.to_string()
    } else {
        extrair_texto_limpo(html)
    };

    let (util, motivo) = analisar_qualidade(&texto, config);
    if !util {
        *filtrados += 1;
        println!("  [FILTRADO] {} — {}", url, motivo);
        return;
    }

    let hash = calcular_hash(&texto);

    match hash_armazenado(client, url) {
        Some(hash_antigo) => {
            if hash_antigo == hash {
                *ignorados += 1;
            } else {
                match atualizar_documento(client, url, &texto, &hash) {
                    Ok(_) => *inseridos += 1,
                    Err(e) => {
                        if e_erro_duplicata(&e) {
                            *ignorados += 1;
                        } else {
                            *erros += 1;
                            eprintln!("  [ERRO UPDATE] {}: {:?}", url, e);
                        }
                    }
                }
            }
        }
        None => {
            match inserir_documento(client, url, domain, doc_type, &texto, &hash) {
                Ok(_) => {
                    *inseridos += 1;
                    println!("  [OK] ({}/{}) {} ({} bytes texto)",
                        inseridos, max_paginas, url, texto.len());
                }
                Err(e) => {
                    if e_erro_duplicata(&e) {
                        *ignorados += 1;
                    } else {
                        *erros += 1;
                        eprintln!("  [ERRO DB] {}: {:?}", url, e);
                    }
                }
            }
        }
    }
}

// =============================================================================
// CRAWLER BFS
// =============================================================================

fn coletar_crawling(
    client: &mut postgres::Client,
    fonte: &FonteConfig,
    config: &Config,
) {
    // max_paginas da fonte sobrescreve o global se definido
    let max_paginas = fonte.max_paginas.unwrap_or(config.max_paginas);

    println!("\n[FONTE] {} ({}) — limite {} páginas", fonte.url, fonte.domain, max_paginas);

    let mut visitados: HashSet<String> = HashSet::new();
    let mut fila: VecDeque<(String, usize)> = VecDeque::new();
    let mut inseridos = 0;
    let mut ignorados_duplicados = 0;
    let mut ignorados_binarios = 0;
    let mut filtrados = 0;
    let mut erros = 0;
    let mut visitadas = 0;

    fila.push_back((canonicalizar_url(fonte.url), 0));

    while let Some((url_atual, profundidade)) = fila.pop_front() {
        if visitados.contains(&url_atual) {
            continue;
        }
        if visitadas >= max_paginas * 3 {
            println!("  [LIMITE] {} paginas visitadas", visitadas);
            break;
        }
        if inseridos >= max_paginas {
            println!("  [LIMITE] {} insercoes atingido", max_paginas);
            break;
        }

        if url_e_binaria(&url_atual) {
            ignorados_binarios += 1;
            visitados.insert(url_atual);
            continue;
        }
        if url_parece_indice(&url_atual) {
            filtrados += 1;
            visitados.insert(url_atual);
            continue;
        }

        visitados.insert(url_atual.clone());
        visitadas += 1;

        match baixar_conteudo(&url_atual, config.max_bytes_html) {
            Ok(html) => {
                let links = if profundidade < config.max_profundidade {
                    extrair_links(&html, fonte.url)
                } else {
                    vec![]
                };

                processar_pagina(
                    client, &url_atual, fonte.domain, fonte.doc_type, &html,
                    &mut inseridos, &mut ignorados_duplicados, &mut filtrados, &mut erros,
                    max_paginas, config,
                );

                for link in links {
                    if !visitados.contains(&link) {
                        fila.push_back((link, profundidade + 1));
                    }
                }
            }
            Err(e) => {
                erros += 1;
                eprintln!("  [ERRO HTTP] {}: {}", url_atual, e);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(config.delay_ms));
    }

    println!("  [RESUMO] visitadas={} inseridos={} filtrados={} ignorados_duplicados={} ignorados_binarios={} erros={}",
        visitadas, inseridos, filtrados, ignorados_duplicados, ignorados_binarios, erros);
}

// =============================================================================
// COLETA NVD — hash apenas dos campos estáveis (corrige re-coleta infinita)
// =============================================================================

/// Extrai apenas os campos estáveis de um lote de CVEs para calcular o hash.
/// Ignora timestamps e outros campos mutáveis que mudam a cada chamada da API.
fn hash_nvd_estavel(json_bruto: &str) -> String {
    // Tenta parsear e extrair apenas campos que não variam
    if let Ok(valor) = serde_json::from_str::<serde_json::Value>(json_bruto) {
        if let Some(vulnerabilidades) = valor["vulnerabilities"].as_array() {
            let campos_estaveis: Vec<serde_json::Value> = vulnerabilidades.iter()
                .filter_map(|v| {
                    let cve = v.get("cve")?;
                    Some(serde_json::json!({
                        "id": cve.get("id"),
                        "descriptions": cve.get("descriptions"),
                        "metrics": cve.get("metrics"),
                        "references": cve.get("references"),
                    }))
                })
                .collect();

            let conteudo_estavel = serde_json::to_string(&campos_estaveis)
                .unwrap_or_else(|_| json_bruto.to_string());
            return calcular_hash(&conteudo_estavel);
        }
    }
    // Fallback: hash do JSON completo se parse falhar
    calcular_hash(json_bruto)
}

fn coletar_nvd(client: &mut postgres::Client) {
    println!("\n[FONTE API] NVD CVE Database (security)");

    let http = reqwest::blocking::Client::builder()
        .user_agent("NEXUS-Agent/0.1")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();

    let mut inseridos = 0;
    let mut ignorados_duplicados = 0;
    let ignorados_binarios = 0;
    let mut erros = 0;
    let por_pagina = 2000;
    let max_cves = 20000;
    let mut inicio = 0;

    loop {
        if inicio >= max_cves { break; }

        let source_url = format!("nvd-api-cves-{}-{}", inicio, inicio + por_pagina);
        let api_url = format!(
            "https://services.nvd.nist.gov/rest/json/cves/2.0?resultsPerPage={}&startIndex={}",
            por_pagina, inicio
        );

        println!("  Buscando CVEs {} a {}...", inicio, inicio + por_pagina);

        let mut tentativa = 0;
        let max_tentativas = 3;

        let resultado = loop {
            tentativa += 1;
            match http.get(&api_url).send() {
                Ok(resp) => {
                    if resp.status().as_u16() == 429 {
                        let aguardar = resp.headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(60);
                        eprintln!("  [RATE LIMIT NVD] aguardando {}s", aguardar);
                        std::thread::sleep(std::time::Duration::from_secs(aguardar));
                        if tentativa >= max_tentativas { break Err("rate limit excedido".to_string()); }
                        continue;
                    }
                    match resp.text() {
                        Ok(t) => break Ok(t),
                        Err(e) => break Err(e.to_string()),
                    }
                }
                Err(e) => {
                    if tentativa < max_tentativas {
                        let delay = 2u64.pow(tentativa as u32);
                        std::thread::sleep(std::time::Duration::from_secs(delay));
                    } else {
                        break Err(e.to_string());
                    }
                }
            }
        };

        match resultado {
            Ok(conteudo) if conteudo.len() >= 100 => {
                // Hash apenas dos campos estáveis — resolve re-coleta infinita por timestamps
                let hash = hash_nvd_estavel(&conteudo);

                match hash_armazenado(client, &source_url) {
                    Some(h) if h == hash => {
                        ignorados_duplicados += 1;
                        println!("  [IGNORADO] {} — sem mudancas nos CVEs", source_url);
                    }
                    Some(_) => {
                        let _ = atualizar_documento(client, &source_url, &conteudo, &hash);
                        inseridos += 1;
                    }
                    None => {
                        match inserir_documento(client, &source_url, "security", "cve", &conteudo, &hash) {
                            Ok(_) => {
                                inseridos += 1;
                                println!("  [OK] {} ({} bytes)", source_url, conteudo.len());
                            }
                            Err(e) => {
                                if e_erro_duplicata(&e) { ignorados_duplicados += 1; }
                                else { erros += 1; eprintln!("  [ERRO DB] {:?}", e); }
                            }
                        }
                    }
                }
            }
            Ok(_) => {
                erros += 1;
                eprintln!("  [ERRO] Resposta vazia ou muito curta para {}", source_url);
            }
            Err(e) => {
                erros += 1;
                eprintln!("  [ERRO HTTP] {}", e);
            }
        }

        inicio += por_pagina;
        std::thread::sleep(std::time::Duration::from_secs(6));
    }

    println!("  [RESUMO NVD] inseridos={} ignorados_duplicados={} ignorados_binarios={} erros={}", inseridos, ignorados_duplicados, ignorados_binarios, erros);
}

// =============================================================================
// MAIN
// =============================================================================

fn main() {
    let senha = std::env::var("KB_INGEST_PASSWORD").unwrap_or_else(|_| {
        eprintln!("[ERRO] Variavel de ambiente KB_INGEST_PASSWORD nao definida.");
        std::process::exit(1);
    });

    let conn_str = format!(
        "host=localhost port=5432 dbname=knowledge_base user=kb_ingest password={}",
        senha
    );

    let mut client = match postgres::Client::connect(&conn_str, postgres::NoTls) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERRO] Falha ao conectar ao banco: {}", e);
            std::process::exit(1);
        }
    };

    println!("Conectado ao banco. Iniciando coleta...\n");

    let config = Config::default();

    // -------------------------------------------------------------------------
    // SECURITY
    // -------------------------------------------------------------------------
    coletar_crawling(&mut client, &FonteConfig::new("https://owasp.org/Top10/2025/", "security", "documentation"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://www.rfc-editor.org/rfc/rfc8446", "security", "rfc"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://www.nist.gov/cyberframework", "security", "documentation"), &config);
    coletar_nvd(&mut client);

    // -------------------------------------------------------------------------
    // RUST
    // -------------------------------------------------------------------------
    coletar_crawling(&mut client, &FonteConfig::new("https://doc.rust-lang.org/stable/reference/", "rust", "documentation"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://doc.rust-lang.org/nomicon/", "rust", "documentation"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://doc.rust-lang.org/book/", "rust", "documentation"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://doc.rust-lang.org/std/", "rust", "documentation"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://doc.rust-lang.org/cargo/", "rust", "documentation"), &config);

    // -------------------------------------------------------------------------
    // INFRA — kernel.org com limite maior por ter muito mais conteúdo
    // -------------------------------------------------------------------------
    coletar_crawling(&mut client,
        &FonteConfig::new("https://www.kernel.org/doc/html/latest/", "infra", "documentation")
            .com_limite(500),
        &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://docs.docker.com/", "infra", "documentation"), &config);
    coletar_crawling(&mut client,
        &FonteConfig::new("https://www.postgresql.org/docs/17/", "infra", "documentation")
            .com_limite(200),
        &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://systemd.io/", "infra", "documentation"), &config);

    // -------------------------------------------------------------------------
    // MLOPS
    // -------------------------------------------------------------------------
    coletar_crawling(&mut client, &FonteConfig::new("https://arxiv.org/abs/2305.14314", "mlops", "paper"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://huggingface.co/docs/peft/", "mlops", "documentation"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://huggingface.co/docs/transformers/", "mlops", "documentation"), &config);
    coletar_crawling(&mut client, &FonteConfig::new("https://raw.githubusercontent.com/ggerganov/llama.cpp/master/README.md", "mlops", "documentation"), &config);

    println!("\nColeta finalizada.");
}
