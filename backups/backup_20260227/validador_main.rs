use postgres::{Client, NoTls};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};

const SESSION_FILE: &str = "nexus_session.txt";
const BATCH_SIZE: i64 = 50;

// ─────────────────────────────────────────────────────────────────────────────

struct Documento {
    id: String,
    source: String,
    domain: String,
    doc_type: String,
    content_length: i32,
    preview: String, // primeiras 800 chars
}

// Ação aplicada a um documento — usada para desfazer com [v]
#[derive(Clone)]
#[allow(dead_code)]
enum Acao {
    Aprovado,
    Rejeitado(String), // motivo
    Inutil,
    Pulado,
}

struct HistoricoItem {
    id: String,
    acao: Acao,
}

// ─────────────────────────────────────────────────────────────────────────────
// Banco de dados
// ─────────────────────────────────────────────────────────────────────────────

fn contar_pendentes(client: &mut Client, pulados: &HashSet<String>) -> i64 {
    // Conta documentos pending excluindo os pulados nesta sessão
    let excluidos: Vec<&str> = pulados.iter().map(|s| s.as_str()).collect();
    if excluidos.is_empty() {
        let row = client
            .query_one(
                "SELECT COUNT(*) FROM documents d \
                 JOIN validation v ON v.document_id = d.id \
                 WHERE v.status = 'pending'",
                &[],
            )
            .expect("Erro ao contar pendentes");
        row.get::<_, i64>(0)
    } else {
        // usa ANY para exclusão dinâmica
        let row = client
            .query_one(
                "SELECT COUNT(*) FROM documents d \
                 JOIN validation v ON v.document_id = d.id \
                 WHERE v.status = 'pending' \
                   AND d.id::text != ALL($1)",
                &[&excluidos],
            )
            .expect("Erro ao contar pendentes");
        row.get::<_, i64>(0)
    }
}

fn buscar_lote(
    client: &mut Client,
    pulados_sessao: &HashSet<String>,
) -> Vec<Documento> {
    let excluidos: Vec<&str> = pulados_sessao.iter().map(|s| s.as_str()).collect();

    let sql_base = "
        SELECT d.id::text, d.source, d.domain, d.doc_type, d.content_length,
               LEFT(convert_from(convert_to(d.content, 'UTF8'), 'UTF8'), 800) as preview
        FROM documents d
        JOIN validation v ON v.document_id = d.id
        WHERE v.status = 'pending'
    ";

    let rows = if excluidos.is_empty() {
        client
            .query(
                &format!("{} ORDER BY d.domain, d.collected_at LIMIT $1", sql_base),
                &[&BATCH_SIZE],
            )
            .expect("Erro ao buscar documentos")
    } else {
        client
            .query(
                &format!(
                    "{} AND d.id::text != ALL($1) ORDER BY d.domain, d.collected_at LIMIT $2",
                    sql_base
                ),
                &[&excluidos, &BATCH_SIZE],
            )
            .expect("Erro ao buscar documentos")
    };

    rows.iter()
        .map(|row| Documento {
            id: row.get(0),
            source: row.get(1),
            domain: row.get(2),
            doc_type: row.get(3),
            content_length: row.get(4),
            preview: row.get(5),
        })
        .collect()
}

fn buscar_conteudo_completo(client: &mut Client, id: &str) -> String {
    let row = client
        .query_one(
            "SELECT convert_from(convert_to(content, 'UTF8'), 'UTF8') \
             FROM documents WHERE id::text = $1",
            &[&id],
        )
        .expect("Erro ao buscar conteúdo completo");
    row.get::<_, String>(0)
}

fn db_aprovar(client: &mut Client, id: &str) {
    client
        .execute(
            "UPDATE validation SET status = 'approved', decided_by = 'human', \
             decided_at = NOW() WHERE document_id::text = $1",
            &[&id],
        )
        .expect("Erro ao aprovar");
}

fn db_rejeitar(client: &mut Client, id: &str, motivo: &str) {
    client
        .execute(
            "UPDATE validation SET status = 'rejected', rejection_reason = $2, \
             decided_by = 'human', decided_at = NOW() \
             WHERE document_id::text = $1",
            &[&id, &motivo],
        )
        .expect("Erro ao rejeitar");
}

fn db_desfazer(client: &mut Client, id: &str) {
    // Volta o documento para 'pending'
    client
        .execute(
            "UPDATE validation SET status = 'pending', decided_by = 'pending', \
             decided_at = NULL, rejection_reason = NULL \
             WHERE document_id::text = $1",
            &[&id],
        )
        .expect("Erro ao desfazer decisão");
}

// ─────────────────────────────────────────────────────────────────────────────
// Sessão (persistência de pulados entre execuções)
// ─────────────────────────────────────────────────────────────────────────────

fn carregar_sessao() -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Ok(conteudo) = fs::read_to_string(SESSION_FILE) {
        for linha in conteudo.lines() {
            let id = linha.trim();
            if !id.is_empty() {
                ids.insert(id.to_string());
            }
        }
    }
    ids
}

fn salvar_pulado_sessao(id: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(SESSION_FILE)
        .expect("Erro ao abrir arquivo de sessão");
    writeln!(file, "{}", id).expect("Erro ao salvar sessão");
}

fn limpar_sessao() {
    let _ = fs::remove_file(SESSION_FILE);
}

// ─────────────────────────────────────────────────────────────────────────────
// Terminal
// ─────────────────────────────────────────────────────────────────────────────

fn limpar_tela() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
}

fn ler_linha(stdin: &io::Stdin) -> String {
    let mut buf = String::new();
    stdin.lock().read_line(&mut buf).unwrap();
    buf.trim().to_lowercase()
}

fn exibir_ajuda() {
    println!();
    println!("  ┌─────────────────────────────────────────────────┐");
    println!("  │  COMANDOS                                        │");
    println!("  │                                                  │");
    println!("  │  [a] Aprovar                                     │");
    println!("  │  [r] Rejeitar  (pede motivo)                     │");
    println!("  │  [u] Inútil    (rejeição rápida, sem digitar)    │");
    println!("  │  [p] Pular     (deixa como pendente, lembra)     │");
    println!("  │  [v] Voltar    (desfaz última decisão)           │");
    println!("  │  [?] Ver mais  (exibe conteúdo completo)         │");
    println!("  │  [s] Salvar e sair                               │");
    println!("  │  [q] Sair      (descarta pulados da sessão)      │");
    println!("  └─────────────────────────────────────────────────┘");
    println!();
}

fn exibir_documento(
    doc: &Documento,
    idx: usize,
    total_lote: usize,
    aprovados: u32,
    rejeitados: u32,
    pulados_count: u32,
    total_pendentes: i64,
    linhas: usize,
) {
    limpar_tela();

    let progresso_total = aprovados + rejeitados;
    let total_geral = progresso_total as i64 + total_pendentes;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  NEXUS — VALIDADOR DE DOCUMENTOS                            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  Lote: {}/{}   Sessão: ✓{}  ✗{}  ~{}   Total: {}/{}{}║",
        idx + 1,
        total_lote,
        aprovados,
        rejeitados,
        pulados_count,
        progresso_total,
        total_geral,
        " ".repeat(
            62usize.saturating_sub(
                format!(
                    "  Lote: {}/{}   Sessão: ✓{}  ✗{}  ~{}   Total: {}/{}",
                    idx + 1,
                    total_lote,
                    aprovados,
                    rejeitados,
                    pulados_count,
                    progresso_total,
                    total_geral
                )
                .len()
            )
        )
    );
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Domínio  : {}", doc.domain);
    println!("  Tipo     : {}", doc.doc_type);
    println!("  Tamanho  : {} bytes", doc.content_length);
    println!("  Fonte    : {}", doc.source);
    println!();
    println!("──────────────────────────────────────────────────────────────");
    println!("  PRÉVIA DO CONTEÚDO ({} linhas):", linhas);
    println!("──────────────────────────────────────────────────────────────");
    println!();

    for linha in doc.preview.lines().take(linhas) {
        println!("  {}", linha);
    }

    println!();
    println!("──────────────────────────────────────────────────────────────");
    println!("  [a] Aprovar  [r] Rejeitar  [u] Inútil  [p] Pular  [s] Salvar  [q] Sair");
    println!("  [i] Sugestão [t] Auto-IA  [v] Voltar   [?] Ver mais");
    println!("──────────────────────────────────────────────────────────────");
    println!();
    print!("  Decisão: ");
    io::stdout().flush().unwrap();
}


// ─────────────────────────────────────────────────────────────────────────────
// Sugestão automática baseada em heurísticas locais
// ─────────────────────────────────────────────────────────────────────────────

struct Sugestao {
    util: bool,
    confianca: u8,
    motivo: String,
}

fn sugerir(doc: &Documento) -> Sugestao {
    let texto = doc.preview.to_lowercase();
    let linhas: Vec<&str> = doc.preview.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let total = linhas.len().max(1);

    // ── Pontuação negativa acumulada ──────────────────────────────────────────
    // Cada sinal ruim soma pontos. Se ultrapassar limiar, rejeita.
    let mut pontos_ruins: i32 = 0;
    let mut motivos_ruins: Vec<String> = Vec::new();

    // Cabeçalho de idioma (sinal fraco — kernel.org tem em quase toda página)
    if texto.contains("chinese (simplified)") || texto.contains("chinese (traditional)") {
        pontos_ruins += 15;
        motivos_ruins.push("cabeçalho de idioma presente".to_string());
    }

    // Linhas curtas demais
    let curtas = linhas.iter().filter(|l| l.len() < 20).count();
    let pct_curtas = (curtas * 100) / total;
    if pct_curtas > 75 {
        pontos_ruins += 40;
        motivos_ruins.push(format!("{}% das linhas < 20 chars", pct_curtas));
    } else if pct_curtas > 55 {
        pontos_ruins += 20;
        motivos_ruins.push(format!("{}% das linhas são curtas", pct_curtas));
    }

    // Linhas duplicadas
    let mut vistos = std::collections::HashSet::new();
    let mut duplicadas = 0usize;
    for l in &linhas {
        if l.len() > 5 && !vistos.insert(*l) {
            duplicadas += 1;
        }
    }
    let pct_dup = (duplicadas * 100) / total;
    if pct_dup > 50 {
        pontos_ruins += 35;
        motivos_ruins.push(format!("{}% das linhas duplicadas", pct_dup));
    } else if pct_dup > 30 {
        pontos_ruins += 15;
        motivos_ruins.push(format!("{}% de duplicatas", pct_dup));
    }

    // Assinaturas fortes de lixo (eliminatórias sozinhas)
    let assinaturas_fortes: &[(&str, &str)] = &[
        ("table of contents\n", "página de TOC puro"),
        ("your account\ndocumentation\n", "cabeçalho postgresql.org"),
        ("next page\nprevious page\n", "paginação de navegação"),
        ("skip to main content\n", "página sem conteúdo real"),
        ("search results for", "página de resultado de busca"),
    ];
    for (assinatura, descricao) in assinaturas_fortes {
        if texto.contains(assinatura) {
            return Sugestao {
                util: false,
                confianca: 95,
                motivo: format!("Eliminatório: {} ({})", descricao, assinatura.trim()),
            };
        }
    }

    // Tamanho muito pequeno
    if doc.content_length < 800 {
        pontos_ruins += 40;
        motivos_ruins.push(format!("apenas {} bytes", doc.content_length));
    } else if doc.content_length < 1500 {
        pontos_ruins += 10;
    }

    // Ausência de pontuação
    let chars_total = texto.len().max(1);
    let pontuacao = texto.chars().filter(|&c| c == '.' || c == ',' || c == ';' || c == ':').count();
    let pct_pont = (pontuacao * 1000) / chars_total;
    if pct_pont < 2 && total > 20 {
        pontos_ruins += 25;
        motivos_ruins.push("quase sem pontuação".to_string());
    }

    // ── Palavras-chave por domínio ────────────────────────────────────────────
    let palavras_rust: &[&str] = &[
        "fn ", "let ", "mut ", "impl ", "trait ", "struct ", "enum ", "match ",
        "borrow", "lifetime", "ownership", "unsafe", "cargo", "crate",
        "async", "await", "result", "option", "closure", "iterator", "generic",
        "compiler", "rustc", "macro", "pattern", "reference", "slice",
    ];
    let palavras_infra: &[&str] = &[
        "kernel", "syscall", "driver", "interrupt", "memory management",
        "process", "thread", "scheduler", "filesystem", "network stack",
        "docker", "container", "volume", "postgres", "sql", "query", "index",
        "systemd", "service unit", "socket", "journal", "cgroup", "namespace",
        "ioctl", "mmap", "fork", "signal", "buffer", "mount", "inode",
        "lock", "mutex", "semaphore", "spinlock", "rcu", "dma",
    ];
    let palavras_security: &[&str] = &[
        "vulnerability", "cve-", "exploit", "injection", "xss", "csrf",
        "authentication", "authorization", "encryption", "tls", "ssl",
        "owasp", "nist", "rfc ", "attack vector", "mitigation",
        "privilege escalation", "sanitize", "certificate", "hash",
        "buffer overflow", "memory corruption", "denial of service",
    ];
    let palavras_mlops: &[&str] = &[
        "fine-tuning", "lora", "qlora", "transformer", "attention mechanism",
        "embedding", "dataset", "gradient", "optimizer", "loss function",
        "huggingface", "pytorch", "llm", "tokenizer", "quantization",
        "inference", "training loop", "batch size", "learning rate",
        "peft", "adapter", "checkpoint", "model weights",
    ];

    let (palavras, nome): (&[&str], &str) = match doc.domain.as_str() {
        "rust"     => (palavras_rust, "Rust"),
        "infra"    => (palavras_infra, "Infra"),
        "security" => (palavras_security, "Security"),
        "mlops"    => (palavras_mlops, "MLOps"),
        _          => (palavras_infra, "geral"),
    };

    let hits: Vec<&str> = palavras.iter()
        .filter(|p| texto.contains(**p))
        .map(|p| p.trim())
        .collect();
    let n = hits.len();

    // ── Decisão final combinando sinais ───────────────────────────────────────

    // Muitas palavras-chave técnicas compensam sinais ruins leves
    let bonus_tecnico: i32 = if n >= 8 { 50 }
        else if n >= 5 { 35 }
        else if n >= 3 { 20 }
        else if n >= 1 { 8 }
        else { 0 };

    let score_final = pontos_ruins - bonus_tecnico;

    if n >= 3 && score_final < 30 {
        let exemplos = hits.iter().take(4)
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>().join(", ");
        let confianca = (55 + n * 4 - (score_final.max(0) as usize)).min(97) as u8;
        return Sugestao {
            util: true,
            confianca,
            motivo: format!("{} palavras-chave de {}: {}{}", n, nome, exemplos,
                if hits.len() > 4 { "..." } else { "" }),
        };
    }

    if score_final >= 50 {
        let motivo = if motivos_ruins.is_empty() {
            "sem conteúdo técnico identificável".to_string()
        } else {
            motivos_ruins.join("; ")
        };
        let confianca = (40 + score_final.min(55)) as u8;
        return Sugestao {
            util: false,
            confianca: confianca.min(95),
            motivo,
        };
    }

    // Zona cinzenta
    if n >= 1 {
        let exemplos = hits.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<_>>().join(", ");
        Sugestao {
            util: true,
            confianca: 48,
            motivo: format!("Sinal misto — palavras: {}; problemas: {}",
                exemplos,
                if motivos_ruins.is_empty() { "nenhum".to_string() } else { motivos_ruins.join(", ") }),
        }
    } else {
        Sugestao {
            util: false,
            confianca: 52,
            motivo: format!("Sem palavras-chave de {}{}",
                nome,
                if motivos_ruins.is_empty() { String::new() } else { format!("; {}", motivos_ruins.join(", ")) }),
        }
    }
}

fn exibir_sugestao(s: &Sugestao) {
    let (icone, label) = if s.util { ("✓", "UTIL  ") } else { ("✗", "INUTEL") };
    let barras = (s.confianca / 10) as usize;
    let barra = format!("{}{}", "█".repeat(barras), "░".repeat(10 - barras));
    let motivo = &s.motivo;

    // Quebra o motivo em linhas de 56 chars sem cortar palavras
    let mut linhas_motivo: Vec<String> = Vec::new();
    let mut linha_atual = String::new();
    for palavra in motivo.split_whitespace() {
        if linha_atual.len() + palavra.len() + 1 > 56 {
            if !linha_atual.is_empty() {
                linhas_motivo.push(linha_atual.clone());
                linha_atual.clear();
            }
        }
        if !linha_atual.is_empty() { linha_atual.push(' '); }
        linha_atual.push_str(palavra);
    }
    if !linha_atual.is_empty() { linhas_motivo.push(linha_atual); }

    println!("  ┌─ SUGESTAO ──────────────────────────────────────────────────┐");
    println!("  │  {} {}   Confianca: {}% [{}]  │", icone, label, s.confianca, barra);
    println!("  │                                                              │");
    for l in &linhas_motivo {
        let padding = 56usize.saturating_sub(l.len());
        println!("  │  {}{}  │", l, " ".repeat(padding));
    }
    println!("  └──────────────────────────────────────────────────────────────┘");
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let senha = std::env::var("KB_INGEST_PASSWORD").unwrap_or_else(|_| {
        eprintln!("[ERRO] KB_INGEST_PASSWORD não definida.");
        std::process::exit(1);
    });

    let conn_str = format!(
        "host=localhost port=5432 dbname=knowledge_base user=kb_ingest password={}",
        senha
    );

    let mut client = Client::connect(&conn_str, NoTls).unwrap_or_else(|e| {
        eprintln!("[ERRO] Falha ao conectar: {}", e);
        std::process::exit(1);
    });

    let stdin = io::stdin();

    // Carrega IDs pulados em sessões anteriores
    let mut pulados_sessao = carregar_sessao();
    let sessao_anterior = pulados_sessao.len();

    let mut aprovados: u32 = 0;
    let mut rejeitados: u32 = 0;
    let mut pulados_conta: u32 = 0;

    // Histórico de decisões para [v] (voltar/desfazer)
    let mut historico: Vec<HistoricoItem> = Vec::new();

    // Toggle de sugestão automática
    let mut sugestao_auto: bool = false;

    if sessao_anterior > 0 {
        limpar_tela();
        println!();
        println!("  Sessão anterior encontrada: {} documentos pulados lembrados.", sessao_anterior);
        println!("  Eles serão omitidos nesta sessão.");
        print!("  Continuar? [Enter] ou limpar sessão [l]: ");
        io::stdout().flush().unwrap();
        let resp = ler_linha(&stdin);
        if resp == "l" {
            limpar_sessao();
            pulados_sessao.clear();
            println!("  Sessão limpa. Começando do zero.");
        }
        println!();
    }

    'principal: loop {
        let _total_pendentes = contar_pendentes(&mut client, &pulados_sessao);
        let docs = buscar_lote(&mut client, &pulados_sessao);

        if docs.is_empty() {
            limpar_tela();
            println!("╔══════════════════════════════════════╗");
            println!("║     VALIDAÇÃO CONCLUÍDA              ║");
            println!("╚══════════════════════════════════════╝");
            println!();
            println!("  Aprovados : {}", aprovados);
            println!("  Rejeitados: {}", rejeitados);
            println!("  Pulados   : {}", pulados_conta);
            println!();
            println!("  Nenhum documento pendente restante.");
            limpar_sessao();
            break;
        }

        let total_lote = docs.len();
        let mut idx: usize = 0;

        while idx < total_lote {
            let doc = &docs[idx];
            let pendentes_restantes = contar_pendentes(&mut client, &pulados_sessao);

            exibir_documento(
                doc,
                idx,
                total_lote,
                aprovados,
                rejeitados,
                pulados_conta,
                pendentes_restantes,
                30,
            );

            if sugestao_auto {
                let s = sugerir(doc);
                exibir_sugestao(&s);
            }

            let decisao = ler_linha(&stdin);

            match decisao.as_str() {
                // ── Aprovar ────────────────────────────────────────────────
                "a" => {
                    db_aprovar(&mut client, &doc.id);
                    historico.push(HistoricoItem {
                        id: doc.id.clone(),
                        acao: Acao::Aprovado,
                    });
                    aprovados += 1;
                    idx += 1;
                }

                // ── Rejeitar com motivo ────────────────────────────────────
                "r" => {
                    print!("  Motivo da rejeição: ");
                    io::stdout().flush().unwrap();
                    let mut motivo_buf = String::new();
                    stdin.lock().read_line(&mut motivo_buf).unwrap();
                    let motivo = motivo_buf.trim().to_string();
                    let motivo = if motivo.is_empty() {
                        "sem motivo especificado".to_string()
                    } else {
                        motivo
                    };
                    db_rejeitar(&mut client, &doc.id, &motivo);
                    historico.push(HistoricoItem {
                        id: doc.id.clone(),
                        acao: Acao::Rejeitado(motivo),
                    });
                    rejeitados += 1;
                    idx += 1;
                }

                // ── Inútil (rejeição rápida) ───────────────────────────────
                "u" => {
                    db_rejeitar(&mut client, &doc.id, "conteúdo inútil");
                    historico.push(HistoricoItem {
                        id: doc.id.clone(),
                        acao: Acao::Inutil,
                    });
                    rejeitados += 1;
                    idx += 1;
                }

                // ── Pular (lembra para não mostrar de novo) ────────────────
                "p" => {
                    salvar_pulado_sessao(&doc.id);
                    pulados_sessao.insert(doc.id.clone());
                    historico.push(HistoricoItem {
                        id: doc.id.clone(),
                        acao: Acao::Pulado,
                    });
                    pulados_conta += 1;
                    idx += 1;
                }

                // ── Voltar / Desfazer última decisão ──────────────────────
                "v" => {
                    if let Some(item) = historico.pop() {
                        match &item.acao {
                            Acao::Aprovado => {
                                db_desfazer(&mut client, &item.id);
                                aprovados = aprovados.saturating_sub(1);
                            }
                            Acao::Rejeitado(_) | Acao::Inutil => {
                                db_desfazer(&mut client, &item.id);
                                rejeitados = rejeitados.saturating_sub(1);
                            }
                            Acao::Pulado => {
                                // Remove da lista de pulados
                                pulados_sessao.remove(&item.id);
                                // Reescreve o arquivo de sessão sem esse ID
                                let novos: Vec<String> =
                                    pulados_sessao.iter().cloned().collect();
                                fs::write(
                                    SESSION_FILE,
                                    novos.join("\n") + if novos.is_empty() { "" } else { "\n" },
                                )
                                .unwrap_or(());
                                pulados_conta = pulados_conta.saturating_sub(1);
                            }
                        }
                        // Volta o índice se o doc desfeito era do lote atual
                        if idx > 0 {
                            idx -= 1;
                        }
                        println!();
                        println!("  ✓ Última decisão desfeita.");
                        std::thread::sleep(std::time::Duration::from_millis(700));
                    } else {
                        println!("  Nenhuma decisão para desfazer.");
                        std::thread::sleep(std::time::Duration::from_millis(700));
                    }
                }

                // ── Toggle sugestão automática ────────────────────────────────
                "t" => {
                    sugestao_auto = !sugestao_auto;
                    let estado = if sugestao_auto { "LIGADA" } else { "DESLIGADA" };
                    println!("  Sugestão automática: {}", estado);
                    std::thread::sleep(std::time::Duration::from_millis(600));
                }

                // ── Sugestão heurística local ──────────────────────────────

                "i" => {
                    let s = sugerir(doc);
                    exibir_sugestao(&s);
                    print!("  [Enter] para continuar: ");
                    io::stdout().flush().unwrap();
                    let mut _buf = String::new();
                    stdin.lock().read_line(&mut _buf).unwrap();
                    stdin.lock().read_line(&mut _buf).unwrap();
                }

                // ── Ver conteudo completo ──────────────────────────────────


                "?" => {
                    let conteudo = buscar_conteudo_completo(&mut client, &doc.id);
                    limpar_tela();
                    println!("══ CONTEÚDO COMPLETO ═══════════════════════════════════════════");
                    println!("  Fonte: {}", doc.source);
                    println!("────────────────────────────────────────────────────────────────");
                    println!();
                    for linha in conteudo.lines() {
                        println!("  {}", linha);
                    }
                    println!();
                    println!("────────────────────────────────────────────────────────────────");
                    print!("  [Enter] para voltar: ");
                    io::stdout().flush().unwrap();
                    let mut _buf = String::new();
                    stdin.lock().read_line(&mut _buf).unwrap();
                    // Não avança idx — mostra o mesmo doc de novo
                }

                // ── Salvar e sair ──────────────────────────────────────────
                "s" => {
                    limpar_tela();
                    println!("  Sessão salva.");
                    println!("  Aprovados={} Rejeitados={} Pulados={}", aprovados, rejeitados, pulados_conta);
                    println!();
                    println!("  {} IDs pulados registrados em '{}'.", pulados_sessao.len(), SESSION_FILE);
                    println!("  Na próxima execução você retomará de onde parou.");
                    break 'principal;
                }

                // ── Sair sem salvar pulados ────────────────────────────────
                "q" => {
                    limpar_tela();
                    print!("  Sair sem salvar pulados desta sessão? Os pulados voltarão como pendentes. [s/N]: ");
                    io::stdout().flush().unwrap();
                    let confirmacao = ler_linha(&stdin);
                    if confirmacao == "s" {
                        limpar_tela();
                        println!("  Sessão encerrada sem salvar pulados.");
                        println!("  Aprovados={} Rejeitados={} Pulados={}", aprovados, rejeitados, pulados_conta);
                        break 'principal;
                    }
                    // Se não confirmou, volta ao documento atual
                }

                // ── Ajuda ──────────────────────────────────────────────────
                "h" | "help" => {
                    exibir_ajuda();
                    print!("  [Enter] para continuar: ");
                    io::stdout().flush().unwrap();
                    let mut _buf = String::new();
                    stdin.lock().read_line(&mut _buf).unwrap();
                }

                _ => {
                    // Comando desconhecido: mostra ajuda inline
                    println!("  Comando '{}' não reconhecido. Use [h] para ver os comandos.", decisao);
                    std::thread::sleep(std::time::Duration::from_millis(800));
                }
            }
        }
    }
}
