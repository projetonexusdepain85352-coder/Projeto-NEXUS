// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::invisible_characters)] // Strings legadas da TUI usam box-drawing e acentos.

use chrono::Local;
use postgres::{Client, NoTls};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// MODO DE SUGESTÃƒÆ’O
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
#[derive(PartialEq, Clone)]
enum ModoSugestao {
    Desligado,
    Heuristica,
    IA,
}
impl ModoSugestao {
    fn label(&self) -> &'static str {
        match self {
            ModoSugestao::Desligado => "DESLIGADO",
            ModoSugestao::Heuristica => "HEURISTICA",
            ModoSugestao::IA => "IA",
        }
    }
}
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// CONFIGURAÃƒâ€¡Ãƒâ€¢ES AVANÃƒâ€¡ADAS
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

const CONFIG_FILE: &str = "nexus_config.json";

struct Config {
    threshold_ia: u8,         // confianÃƒÂ§a mÃƒÂ­nima para auto-aprovar (padrÃƒÂ£o 80)
    threshold_heuristica: u8, // abaixo disso pede decisÃƒÂ£o manual (padrÃƒÂ£o 60)
    timeout_ollama: u64,      // segundos antes de desistir (padrÃƒÂ£o 30)
    tamanho_lote: i64,        // documentos por busca (padrÃƒÂ£o 50)
    linhas_preview: usize,    // linhas exibidas na tela (padrÃƒÂ£o 30)
}

impl Default for Config {
    fn default() -> Self {
        Config {
            threshold_ia: 80,
            threshold_heuristica: 60,
            timeout_ollama: 30,
            tamanho_lote: 50,
            linhas_preview: 30,
        }
    }
}

fn carregar_config() -> Config {
    let mut config = Config::default();
    if let Ok(s) = fs::read_to_string(CONFIG_FILE)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&s)
    {
        if let Some(x) = v["threshold_ia"].as_u64() {
            config.threshold_ia = x as u8;
        }
        if let Some(x) = v["threshold_heuristica"].as_u64() {
            config.threshold_heuristica = x as u8;
        }
        if let Some(x) = v["timeout_ollama"].as_u64() {
            config.timeout_ollama = x;
        }
        if let Some(x) = v["tamanho_lote"].as_i64() {
            config.tamanho_lote = x;
        }
        if let Some(x) = v["linhas_preview"].as_u64() {
            config.linhas_preview = x as usize;
        }
    }
    config
}

fn salvar_config(config: &Config) {
    let json = serde_json::json!({
        "threshold_ia":         config.threshold_ia,
        "threshold_heuristica": config.threshold_heuristica,
        "timeout_ollama":       config.timeout_ollama,
        "tamanho_lote":         config.tamanho_lote,
        "linhas_preview":       config.linhas_preview,
    });
    fs::write(CONFIG_FILE, json.to_string()).unwrap_or_else(|e| eprintln!("[AVISO] config: {}", e));
}

fn revalidar_ia(client: &mut Client, config: &Config, stdin: &io::Stdin) {
    let total: i64 = client
        .query_one("SELECT COUNT(*) FROM validation WHERE decided_by='ai'", &[])
        .map(|r| r.get::<_, i64>(0))
        .unwrap_or(0);

    if total == 0 {
        println!("  Nenhum documento validado por IA encontrado.");
        std::thread::sleep(std::time::Duration::from_millis(1000));
        return;
    }

    println!("  {} documentos serao revalidados pelo Ollama.", total);
    println!(
        "  Tempo estimado: ~{}s por documento.",
        config.timeout_ollama
    );
    print!("  Confirmar? [s/N]: ");
    io::stdout().flush().ok();
    let mut buf = String::new();
    stdin.lock().read_line(&mut buf).ok();
    if buf.trim().to_lowercase() != "s" {
        println!("  Cancelado.");
        std::thread::sleep(std::time::Duration::from_millis(600));
        return;
    }

    let rows = client
        .query(
            "SELECT d.id::text, d.source, d.domain,
                LEFT(convert_from(convert_to(d.content,'UTF8'),'UTF8'), 4000)
         FROM documents d JOIN validation v ON v.document_id = d.id
         WHERE v.decided_by = 'ai'
         ORDER BY v.decided_at",
            &[],
        )
        .expect("Erro ao buscar docs IA");

    let mut n_aprovados = 0usize;
    let mut n_rejeitados = 0usize;
    let mut n_incerto = 0usize;
    let mut pulados_retry: Vec<(String, String, String, String)> = Vec::new();

    // Thread ÃƒÅ¡NICO de stdin para toda a funÃƒÂ§ÃƒÂ£o Ã¢â‚¬â€ elimina race condition
    // O mesmo canal serve para: x no loop principal, x no retry, decisÃƒÂµes manuais
    let parar = Arc::new(AtomicBool::new(false));
    let (tx_stdin, rx_stdin) = std::sync::mpsc::channel::<String>();
    let parar_t = Arc::clone(&parar);
    std::thread::spawn(move || {
        loop {
            let mut b = String::new();
            io::stdin().lock().read_line(&mut b).ok();
            let v = b.trim().to_lowercase();
            if v == "x" {
                parar_t.store(true, Ordering::Relaxed);
            }
            if tx_stdin.send(v).is_err() {
                break;
            }
        }
    });

    println!("  [x+Enter] para interromper.");
    println!();

    // Ã¢â€â‚¬Ã¢â€â‚¬ Loop principal Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    for (i, row) in rows.iter().enumerate() {
        if parar.load(Ordering::Relaxed) {
            println!("  Interrompido pelo usuario.");
            break;
        }
        let id: String = row.get(0);
        let source: String = row.get(1);
        let domain: String = row.get(2);
        let raw: String = row.get(3);
        let preview = filtrar_preview(&raw);

        let doc_tmp = Documento {
            id: id.clone(),
            source: source.clone(),
            domain: domain.clone(),
            doc_type: String::new(),
            content_length: raw.len() as i32,
            preview: preview.clone(),
            content: String::new(),
            head: String::new(),
        };
        let heur = sugerir_heuristica_interna(&doc_tmp);

        let n_linhas_raw = raw.lines().count();
        exibir_documento_revalidar(&doc_tmp, i, total as usize, n_linhas_raw);
        print!("  [IA] Processando (x+Enter para parar)");
        io::stdout().flush().ok();

        let timeout_s = config.timeout_ollama;
        let dom_clone = domain.clone();
        let prev_clone = preview.clone();
        let handle = std::thread::spawn(move || sugerir_ia(&dom_clone, &prev_clone, timeout_s));
        let max_dots = (timeout_s * 1000 / 300 + 10) as usize;
        let mut dots = 0usize;
        loop {
            if handle.is_finished() {
                break;
            }
            if parar.load(Ordering::Relaxed) {
                break;
            }
            while rx_stdin.try_recv().is_ok() {}
            std::thread::sleep(std::time::Duration::from_millis(300));
            print!(".");
            io::stdout().flush().ok();
            dots += 1;
            if dots > max_dots {
                break;
            }
        }
        // 150ms para o thread processar x digitado no mesmo instante que a IA terminou
        std::thread::sleep(std::time::Duration::from_millis(150));
        while rx_stdin.try_recv().is_ok() {}

        if parar.load(Ordering::Relaxed) {
            println!(" Interrompido.");
            break;
        }

        match handle.join().ok().flatten() {
            None => {
                println!();
                println!("  [IA] Sem resposta.");
                n_incerto += 1;
                pulados_retry.push((id.clone(), source.clone(), domain.clone(), raw.clone()));
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            Some(s) => {
                println!();
                exibir_resultado_revalidar(&s, &heur);

                if heur.confianca < config.threshold_heuristica {
                    // HeurÃƒÂ­stica baixa Ã¢â‚¬â€ pede decisÃƒÂ£o manual por 30s
                    let util_ia = s.categoria == Categoria::Util;
                    let sugestao_tmp = Sugestao {
                        categoria: if util_ia {
                            Categoria::Util
                        } else {
                            Categoria::Inutil
                        },
                        confianca: s.confianca,
                        motivo: s.motivo.clone(),
                    };
                    exibir_sugestao(&sugestao_tmp);
                    loop {
                        print!(
                            "  [HEU] Heuristica baixa ({}%) Ã¢â‚¬â€ sua decisao em 30s [a/r/u/?/i] ou pula: ",
                            heur.confianca
                        );
                        io::stdout().flush().ok();
                        match rx_stdin
                            .recv_timeout(std::time::Duration::from_secs(30))
                            .unwrap_or_default()
                            .as_str()
                        {
                            "?" => {
                                exibir_conteudo_completo_rx(&raw, &source, &rx_stdin);
                                continue;
                            }
                            "i" => {
                                exibir_sugestao(&heur);
                                continue;
                            }
                            "x" => {
                                println!("  -> Interrompido.");
                                break;
                            }
                            "a" => {
                                db_aprovar_ia(client, &id);
                                let mut tags = gerar_tags_por_url(&source, &domain, true);
                                tags.push(format!("heur:{}", heur.confianca));
                                tags.dedup();
                                let arr: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
                                client
                                    .execute(
                                        "UPDATE validation SET tags=$2 WHERE document_id::text=$1",
                                        &[&id, &arr],
                                    )
                                    .unwrap_or_else(|e| {
                                        eprintln!("[AVISO] {}", e);
                                        0
                                    });
                                n_aprovados += 1;
                                println!("  -> Aprovado.");
                                break;
                            }
                            "r" | "u" => {
                                let motivo = "[ia-rev-manual] heuristica baixa".to_string();
                                db_rejeitar_ia(client, &id, &motivo);
                                let mut tags = gerar_tags_por_url(&source, &domain, false);
                                tags.push(format!("heur:{}", heur.confianca));
                                tags.dedup();
                                let arr: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
                                client
                                    .execute(
                                        "UPDATE validation SET tags=$2 WHERE document_id::text=$1",
                                        &[&id, &arr],
                                    )
                                    .unwrap_or_else(|e| {
                                        eprintln!("[AVISO] {}", e);
                                        0
                                    });
                                n_rejeitados += 1;
                                println!("  -> Rejeitado.");
                                break;
                            }
                            _ => {
                                println!("  -> Sem resposta Ã¢â‚¬â€ pulando.");
                                n_incerto += 1;
                                break;
                            }
                        }
                    }
                } else if s.confianca >= config.threshold_ia {
                    let util = s.categoria == Categoria::Util;
                    let motivo_ia = format!("[ia-rev] {}", s.motivo);
                    let mut tags = gerar_tags_por_url(&source, &domain, util);
                    tags.push(format!("heur:{}", heur.confianca));
                    tags.dedup();
                    let arr: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
                    if util {
                        db_aprovar_ia(client, &id);
                        n_aprovados += 1;
                    } else {
                        db_rejeitar_ia(client, &id, &motivo_ia);
                        n_rejeitados += 1;
                    }
                    client
                        .execute(
                            "UPDATE validation SET tags=$2 WHERE document_id::text=$1",
                            &[&id, &arr],
                        )
                        .unwrap_or_else(|e| {
                            eprintln!("[AVISO] {}", e);
                            0
                        });
                    std::thread::sleep(std::time::Duration::from_millis(400));
                } else {
                    // ConfianÃƒÂ§a IA baixa Ã¢â‚¬â€ mantido/incerto
                    n_incerto += 1;
                    std::thread::sleep(std::time::Duration::from_millis(400));
                }
            }
        }
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ RETRY: docs sem resposta Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    if !pulados_retry.is_empty() && !parar.load(Ordering::Relaxed) {
        println!();
        print!(
            "  {} documento(s) sem resposta. Tentar novamente? [s/N]: ",
            pulados_retry.len()
        );
        io::stdout().flush().ok();
        let resp_retry = rx_stdin
            .recv_timeout(std::time::Duration::from_secs(60))
            .unwrap_or_default();

        if resp_retry.trim() == "s" {
            let mut fila_manual: Vec<(String, String, String, String)> = Vec::new();

            println!("  [x+Enter] para interromper a qualquer momento.");
            println!();
            let total_r = pulados_retry.len();

            for (ri, (r_id, r_source, r_domain, r_raw)) in pulados_retry.iter().enumerate() {
                if parar.load(Ordering::Relaxed) {
                    break;
                }

                let r_preview = filtrar_preview(r_raw);
                let doc_tmp = Documento {
                    id: r_id.clone(),
                    source: r_source.clone(),
                    domain: r_domain.clone(),
                    doc_type: String::new(),
                    content_length: r_raw.len() as i32,
                    preview: r_preview.clone(),
                    content: String::new(),
                    head: String::new(),
                };
                let heur = sugerir_heuristica_interna(&doc_tmp);

                print!("  [retry {}/{}] {:.60}  ", ri + 1, total_r, r_source);
                io::stdout().flush().ok();

                let timeout_s = config.timeout_ollama;
                let dom_c = r_domain.clone();
                let prev_c = r_preview.clone();
                let handle = std::thread::spawn(move || sugerir_ia(&dom_c, &prev_c, timeout_s));
                let max_dots = (timeout_s * 1000 / 300 + 10) as usize;
                let mut dots = 0usize;
                loop {
                    if handle.is_finished() {
                        break;
                    }
                    if parar.load(Ordering::Relaxed) {
                        break;
                    }
                    while rx_stdin.try_recv().is_ok() {}
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    print!(".");
                    io::stdout().flush().ok();
                    dots += 1;
                    if dots > max_dots {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(150));
                while rx_stdin.try_recv().is_ok() {}

                if parar.load(Ordering::Relaxed) {
                    println!();
                    println!("  Interrompido.");
                    break;
                }

                match handle.join().ok().flatten() {
                    None => {
                        println!(" sem resposta (retry)");
                        fila_manual.push((
                            r_id.clone(),
                            r_source.clone(),
                            r_domain.clone(),
                            r_raw.clone(),
                        ));
                    }
                    Some(s) if heur.confianca < config.threshold_heuristica => {
                        println!();
                        let util_ia = s.categoria == Categoria::Util;
                        let sugestao_tmp = Sugestao {
                            categoria: if util_ia {
                                Categoria::Util
                            } else {
                                Categoria::Inutil
                            },
                            confianca: s.confianca,
                            motivo: s.motivo.clone(),
                        };
                        exibir_sugestao(&sugestao_tmp);
                        loop {
                            print!(
                                "  [HEU] Heuristica baixa ({}%) Ã¢â‚¬â€ sua decisao em 30s [a/r/u/?/i] ou pula: ",
                                heur.confianca
                            );
                            io::stdout().flush().ok();
                            match rx_stdin
                                .recv_timeout(std::time::Duration::from_secs(30))
                                .unwrap_or_default()
                                .as_str()
                            {
                                "?" => {
                                    exibir_conteudo_completo_rx(r_raw, r_source, &rx_stdin);
                                    continue;
                                }
                                "i" => {
                                    exibir_sugestao(&heur);
                                    continue;
                                }
                                "x" => {
                                    println!("  -> Interrompido.");
                                    break;
                                }
                                "a" => {
                                    db_aprovar_ia(client, r_id);
                                    let mut tags = gerar_tags_por_url(r_source, r_domain, true);
                                    tags.push(format!("heur:{}", heur.confianca));
                                    tags.dedup();
                                    let arr: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
                                    client.execute("UPDATE validation SET tags=$2 WHERE document_id::text=$1", &[r_id, &arr]).unwrap_or_else(|e| { eprintln!("[AVISO] {}", e); 0 });
                                    n_aprovados += 1;
                                    println!("  -> Aprovado.");
                                    break;
                                }
                                "r" | "u" => {
                                    let motivo = "[ia-rev-manual] heuristica baixa".to_string();
                                    db_rejeitar_ia(client, r_id, &motivo);
                                    let mut tags = gerar_tags_por_url(r_source, r_domain, false);
                                    tags.push(format!("heur:{}", heur.confianca));
                                    tags.dedup();
                                    let arr: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
                                    client.execute("UPDATE validation SET tags=$2 WHERE document_id::text=$1", &[r_id, &arr]).unwrap_or_else(|e| { eprintln!("[AVISO] {}", e); 0 });
                                    n_rejeitados += 1;
                                    println!("  -> Rejeitado.");
                                    break;
                                }
                                _ => {
                                    println!("  -> Sem resposta Ã¢â‚¬â€ voltara depois.");
                                    fila_manual.push((
                                        r_id.clone(),
                                        r_source.clone(),
                                        r_domain.clone(),
                                        r_raw.clone(),
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                    Some(s) if s.confianca >= config.threshold_ia => {
                        let util = s.categoria == Categoria::Util;
                        let motivo_ia = format!("[ia-rev] {}", s.motivo);
                        let mut tags = gerar_tags_por_url(r_source, r_domain, util);
                        tags.push(format!("heur:{}", heur.confianca));
                        tags.dedup();
                        let arr: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
                        if util {
                            db_aprovar_ia(client, r_id);
                            n_aprovados += 1;
                            println!(" APROVADO ({}%)", heur.confianca);
                        } else {
                            db_rejeitar_ia(client, r_id, &motivo_ia);
                            n_rejeitados += 1;
                            println!(" REJEITADO ({}%)", heur.confianca);
                        }
                        client
                            .execute(
                                "UPDATE validation SET tags=$2 WHERE document_id::text=$1",
                                &[r_id, &arr],
                            )
                            .unwrap_or_else(|e| {
                                eprintln!("[AVISO] {}", e);
                                0
                            });
                    }
                    Some(s) => {
                        println!(" confianca baixa ({}%) Ã¢â‚¬â€ mantido", s.confianca);
                        n_incerto += 1;
                    }
                }
            }

            // Ã¢â€â‚¬Ã¢â€â‚¬ loop manual Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
            while !fila_manual.is_empty() && !parar.load(Ordering::Relaxed) {
                println!();
                println!(
                    "  {} documento(s) aguardando decisao manual:",
                    fila_manual.len()
                );
                let mut respondidos: Vec<usize> = Vec::new();

                for (fi, (f_id, f_source, f_domain, f_raw)) in fila_manual.iter().enumerate() {
                    if parar.load(Ordering::Relaxed) {
                        break;
                    }

                    let f_preview = filtrar_preview(f_raw);
                    let doc_tmp = Documento {
                        id: f_id.clone(),
                        source: f_source.clone(),
                        domain: f_domain.clone(),
                        doc_type: String::new(),
                        content_length: f_raw.len() as i32,
                        preview: f_preview.clone(),
                        content: String::new(),
                        head: String::new(),
                    };
                    let heur = sugerir_heuristica_interna(&doc_tmp);

                    println!("  [{}/{}] {}", fi + 1, fila_manual.len(), f_source);
                    println!("  Heur: {}% | Dominio: {}", heur.confianca, f_domain);
                    print!("  [a] Aprovar  [r/u] Rejeitar  [x] Parar  (30s): ");
                    io::stdout().flush().ok();

                    match rx_stdin
                        .recv_timeout(std::time::Duration::from_secs(30))
                        .unwrap_or_default()
                        .as_str()
                    {
                        "x" => {
                            println!("  -> Interrompido.");
                            break;
                        }
                        "a" => {
                            db_aprovar_ia(client, f_id);
                            let mut tags = gerar_tags_por_url(f_source, f_domain, true);
                            tags.push(format!("heur:{}", heur.confianca));
                            tags.dedup();
                            let arr: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
                            client
                                .execute(
                                    "UPDATE validation SET tags=$2 WHERE document_id::text=$1",
                                    &[f_id, &arr],
                                )
                                .unwrap_or_else(|e| {
                                    eprintln!("[AVISO] {}", e);
                                    0
                                });
                            n_aprovados += 1;
                            respondidos.push(fi);
                            println!("  -> Aprovado.");
                        }
                        "r" | "u" => {
                            let motivo = "[ia-rev-manual] heuristica baixa".to_string();
                            db_rejeitar_ia(client, f_id, &motivo);
                            let mut tags = gerar_tags_por_url(f_source, f_domain, false);
                            tags.push(format!("heur:{}", heur.confianca));
                            tags.dedup();
                            let arr: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
                            client
                                .execute(
                                    "UPDATE validation SET tags=$2 WHERE document_id::text=$1",
                                    &[f_id, &arr],
                                )
                                .unwrap_or_else(|e| {
                                    eprintln!("[AVISO] {}", e);
                                    0
                                });
                            n_rejeitados += 1;
                            respondidos.push(fi);
                            println!("  -> Rejeitado.");
                        }
                        _ => {
                            println!("  -> Sem resposta Ã¢â‚¬â€ proxima rodada.");
                        }
                    }
                }
                respondidos.sort_unstable_by(|a, b| b.cmp(a));
                for idx in respondidos {
                    fila_manual.remove(idx);
                }
            }
        }
    }

    println!();
    println!("  Revalidacao concluida:");
    println!("    Aprovados        : {}", n_aprovados);
    println!("    Rejeitados       : {}", n_rejeitados);
    println!("    Mantidos/incerto : {}", n_incerto);
    print!("  [Enter] para fechar: ");
    io::stdout().flush().ok();
    // Usa o canal para nÃƒÂ£o competir com o thread de stdin ainda ativo
    rx_stdin
        .recv_timeout(std::time::Duration::from_secs(300))
        .ok();
}

fn config_tui(config: &mut Config, client: &mut Client, stdin: &io::Stdin) {
    loop {
        print!("[2J[1;1H");
        io::stdout().flush().ok();
        println!("Ã¢â€¢â€Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢â€”");
        println!("Ã¢â€¢â€˜  NEXUS Ã¢â‚¬â€ CONFIGURAÃƒâ€¡Ãƒâ€¢ES AVANÃƒâ€¡ADAS                             Ã¢â€¢â€˜");
        println!("Ã¢â€¢Å¡Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â");
        println!();
        println!(
            "  [1] Threshold IA (auto-aprovar)  : {}%",
            config.threshold_ia
        );
        println!(
            "  [2] Threshold HeurÃƒÂ­stica (manual): {}%",
            config.threshold_heuristica
        );
        println!(
            "  [3] Timeout Ollama               : {}s",
            config.timeout_ollama
        );
        println!(
            "  [4] Tamanho do lote              : {} docs",
            config.tamanho_lote
        );
        println!(
            "  [5] Linhas de preview            : {}",
            config.linhas_preview
        );
        println!();
        println!("  [ria] Revalidar auto IA  : roda Ollama em todos os docs validados por IA");
        println!();
        println!("  [r] Restaurar padrÃƒÂµes  [s] Salvar e sair  [q] Sair sem salvar");
        println!();
        print!("  OpÃƒÂ§ÃƒÂ£o: ");
        io::stdout().flush().ok();

        let cmd = {
            let mut buf = String::new();
            stdin.lock().read_line(&mut buf).ok();
            buf.trim().to_lowercase()
        };

        match cmd.as_str() {
            "1" => {
                print!(
                    "  Threshold IA [atual: {}%] novo valor (50-99): ",
                    config.threshold_ia
                );
                io::stdout().flush().ok();
                let mut buf = String::new();
                stdin.lock().read_line(&mut buf).ok();
                if let Ok(v) = buf.trim().parse::<u8>() {
                    if (50..=99).contains(&v) {
                        config.threshold_ia = v;
                    } else {
                        println!("  Valor fora do intervalo.");
                        std::thread::sleep(std::time::Duration::from_millis(800));
                    }
                }
            }
            "2" => {
                print!(
                    "  Threshold HeurÃƒÂ­stica [atual: {}%] novo valor (20-90): ",
                    config.threshold_heuristica
                );
                io::stdout().flush().ok();
                let mut buf = String::new();
                stdin.lock().read_line(&mut buf).ok();
                if let Ok(v) = buf.trim().parse::<u8>() {
                    if (20..=90).contains(&v) {
                        config.threshold_heuristica = v;
                    } else {
                        println!("  Valor fora do intervalo.");
                        std::thread::sleep(std::time::Duration::from_millis(800));
                    }
                }
            }
            "3" => {
                print!(
                    "  Timeout Ollama [atual: {}s] novo valor (10-120): ",
                    config.timeout_ollama
                );
                io::stdout().flush().ok();
                let mut buf = String::new();
                stdin.lock().read_line(&mut buf).ok();
                if let Ok(v) = buf.trim().parse::<u64>() {
                    if (10..=120).contains(&v) {
                        config.timeout_ollama = v;
                    } else {
                        println!("  Valor fora do intervalo.");
                        std::thread::sleep(std::time::Duration::from_millis(800));
                    }
                }
            }
            "4" => {
                print!(
                    "  Tamanho do lote [atual: {}] novo valor (10-200): ",
                    config.tamanho_lote
                );
                io::stdout().flush().ok();
                let mut buf = String::new();
                stdin.lock().read_line(&mut buf).ok();
                if let Ok(v) = buf.trim().parse::<i64>() {
                    if (10..=200).contains(&v) {
                        config.tamanho_lote = v;
                    } else {
                        println!("  Valor fora do intervalo.");
                        std::thread::sleep(std::time::Duration::from_millis(800));
                    }
                }
            }
            "5" => {
                print!(
                    "  Linhas de preview [atual: {}] novo valor (10-60): ",
                    config.linhas_preview
                );
                io::stdout().flush().ok();
                let mut buf = String::new();
                stdin.lock().read_line(&mut buf).ok();
                if let Ok(v) = buf.trim().parse::<usize>() {
                    if (10..=60).contains(&v) {
                        config.linhas_preview = v;
                    } else {
                        println!("  Valor fora do intervalo.");
                        std::thread::sleep(std::time::Duration::from_millis(800));
                    }
                }
            }
            "ria" => {
                revalidar_ia(client, config, stdin);
            }
            "r" => {
                *config = Config::default();
                println!("  PadrÃƒÂµes restaurados.");
                std::thread::sleep(std::time::Duration::from_millis(600));
            }
            "s" => {
                salvar_config(config);
                println!("  ConfiguraÃƒÂ§ÃƒÂµes salvas.");
                std::thread::sleep(std::time::Duration::from_millis(600));
                break;
            }
            "q" | "" => break,
            _ => {}
        }
    }
}

const SESSION_FILE: &str = "nexus_session.txt";
const SESSION_STATE_FILE: &str = "nexus_session_state.json";

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// ESTRUTURAS DE DADOS
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

pub struct Documento {
    pub id: String,
    pub source: String,
    pub domain: String,
    pub doc_type: String,
    pub content_length: i32,
    pub preview: String,
    pub content: String,
    pub head: String,
}

enum Acao {
    Aprovado,
    Rejeitado,
    Inutil,
    Pulado,
}

struct HistoricoItem {
    pub id: String,
    acao: Acao,
}

struct EstadoSessao {
    started_at: String,
    aprovados: u32,
    rejeitados: u32,
    pulados: u32,
    ultimo_documento_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Categoria {
    Util,
    Inutil,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sugestao {
    pub categoria: Categoria,
    pub confianca: u8,
    pub motivo: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationDecision {
    Approve,
    Reject { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    EmptyDocument,
    MalformedDocument(String),
    NotPending(ValidationStatus),
    Store(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::EmptyDocument => write!(f, "documento vazio"),
            ValidationError::MalformedDocument(msg) => write!(f, "documento malformado: {}", msg),
            ValidationError::NotPending(status) => write!(f, "status invalido para decisao: {:?}", status),
            ValidationError::Store(msg) => write!(f, "erro de persistencia: {}", msg),
        }
    }
}

impl Error for ValidationError {}

pub trait ValidationStore {
    fn get_status(&mut self, doc_id: &str) -> Result<ValidationStatus, ValidationError>;
    fn update_status(
        &mut self,
        doc_id: &str,
        status: ValidationStatus,
        decided_by: &str,
        reason: Option<&str>,
    ) -> Result<(), ValidationError>;
}

impl ValidationStore for Client {
    fn get_status(&mut self, doc_id: &str) -> Result<ValidationStatus, ValidationError> {
        let row = self
            .query_opt(
                "SELECT status FROM validation WHERE document_id::text = $1",
                &[&doc_id],
            )
            .map_err(|e| ValidationError::Store(e.to_string()))?;
        let status: String = row
            .ok_or_else(|| ValidationError::MalformedDocument("status nao encontrado".to_string()))?
            .get(0);
        match status.as_str() {
            "pending" => Ok(ValidationStatus::Pending),
            "approved" => Ok(ValidationStatus::Approved),
            "rejected" => Ok(ValidationStatus::Rejected),
            other => Err(ValidationError::MalformedDocument(format!(
                "status desconhecido: {}",
                other
            ))),
        }
    }

    fn update_status(
        &mut self,
        doc_id: &str,
        status: ValidationStatus,
        decided_by: &str,
        reason: Option<&str>,
    ) -> Result<(), ValidationError> {
        match status {
            ValidationStatus::Approved => {
                self
                    .execute(
                        "UPDATE validation SET status = 'approved', decided_by = $2, decided_at = NOW(), rejection_reason = NULL WHERE document_id::text = $1",
                        &[&doc_id, &decided_by],
                    )
                    .map_err(|e| ValidationError::Store(e.to_string()))?;
            }
            ValidationStatus::Rejected => {
                let motivo = reason.unwrap_or("");
                self
                    .execute(
                        "UPDATE validation SET status = 'rejected', decided_by = $2, rejection_reason = $3, decided_at = NOW() WHERE document_id::text = $1",
                        &[&doc_id, &decided_by, &motivo],
                    )
                    .map_err(|e| ValidationError::Store(e.to_string()))?;
            }
            ValidationStatus::Pending => {
                return Err(ValidationError::MalformedDocument(
                    "nao e permitido persistir como pending".to_string(),
                ));
            }
        }
        Ok(())
    }
}

pub fn apply_decision<S: ValidationStore>(
    store: &mut S,
    doc_id: &str,
    decision: ValidationDecision,
    decided_by: &str,
) -> Result<ValidationStatus, ValidationError> {
    let current = store.get_status(doc_id)?;
    if current != ValidationStatus::Pending {
        return Err(ValidationError::NotPending(current));
    }

    match decision {
        ValidationDecision::Approve => {
            store.update_status(doc_id, ValidationStatus::Approved, decided_by, None)?;
            Ok(ValidationStatus::Approved)
        }
        ValidationDecision::Reject { reason } => {
            if reason.trim().is_empty() {
                return Err(ValidationError::MalformedDocument(
                    "motivo de rejeicao vazio".to_string(),
                ));
            }
            store.update_status(
                doc_id,
                ValidationStatus::Rejected,
                decided_by,
                Some(&reason),
            )?;
            Ok(ValidationStatus::Rejected)
        }
    }
}

pub fn validate_document_input(doc: &Documento) -> Result<(), ValidationError> {
    if doc.id.trim().is_empty() {
        return Err(ValidationError::MalformedDocument("id vazio".to_string()));
    }
    if doc.source.trim().is_empty() {
        return Err(ValidationError::MalformedDocument("source vazio".to_string()));
    }
    if doc.domain.trim().is_empty() {
        return Err(ValidationError::MalformedDocument("domain vazio".to_string()));
    }
    if doc.content.trim().is_empty() {
        return Err(ValidationError::EmptyDocument);
    }
    Ok(())
}

pub trait SuggestionProvider {
    fn suggest(&self, doc: &Documento) -> Result<Sugestao, ValidationError>;
}

pub struct SugestorSocketProvider {
    socket_path: String,
}

impl SugestorSocketProvider {
    pub fn from_env() -> Option<Self> {
        let socket_path = std::env::var("NEXUS_SUGESTOR_SOCKET").ok()?;
        if socket_path.trim().is_empty() {
            None
        } else {
            Some(Self { socket_path })
        }
    }
}

impl SuggestionProvider for SugestorSocketProvider {
    fn suggest(&self, doc: &Documento) -> Result<Sugestao, ValidationError> {
        #[cfg(not(unix))]
        {
            let _ = doc;
            return Err(ValidationError::Store(
                "sugestor socket indisponivel nesta plataforma".to_string(),
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            let mut stream = UnixStream::connect(&self.socket_path)
                .map_err(|e| ValidationError::Store(format!("sugestor connect: {}", e)))?;
            let payload = serde_json::json!({
                "domain": doc.domain,
                "content": doc.content,
            });
            let msg = format!("{}\n", payload);
            stream
                .write_all(msg.as_bytes())
                .map_err(|e| ValidationError::Store(format!("sugestor write: {}", e)))?;
            let mut resp = String::new();
            stream
                .read_to_string(&mut resp)
                .map_err(|e| ValidationError::Store(format!("sugestor read: {}", e)))?;
            let parsed: serde_json::Value = serde_json::from_str(resp.trim())
                .map_err(|e| ValidationError::Store(format!("sugestor parse: {}", e)))?;
            let util = parsed["util"].as_bool().unwrap_or(false);
            let confianca = parsed["confianca"].as_u64().unwrap_or(0) as u8;
            let motivo = parsed["motivo"].as_str().unwrap_or("sem motivo").to_string();
            Ok(Sugestao {
                categoria: if util { Categoria::Util } else { Categoria::Inutil },
                confianca,
                motivo,
            })
        }
    }
}

pub fn obter_sugestao(
    provider: &dyn SuggestionProvider,
    doc: &Documento,
) -> Result<Sugestao, ValidationError> {
    provider.suggest(doc)
}

pub fn formatar_sugestao(s: &Sugestao) -> String {
    let barras = (s.confianca / 10) as usize;
    let barra = format!("{}{}", "Ã¢â€“Ë†".repeat(barras), "Ã¢â€“â€˜".repeat(10 - barras));
    let linhas_motivo = quebrar_motivo(&s.motivo, 56);
    let (icone, label) = if s.categoria == Categoria::Util {
        ("Ã¢Å“â€œ", "UTIL  ")
    } else {
        ("Ã¢Å“â€”", "INUTEL")
    };

    let mut out = String::new();
    out.push_str("  Ã¢â€Å’Ã¢â€â‚¬ SUGESTAO Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â\n");
    out.push_str(&format!(
        "  Ã¢â€â€š  {} {}   Confianca: {}% [{}]  Ã¢â€â€š\n",
        icone, label, s.confianca, barra
    ));
    out.push_str("  Ã¢â€â€š                                                              Ã¢â€â€š\n");
    for l in &linhas_motivo {
        let p = 56usize.saturating_sub(largura_visual(l));
        out.push_str(&format!("  Ã¢â€â€š  {}{}  Ã¢â€â€š\n", l, " ".repeat(p)));
    }
    out.push_str("  Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ\n");
    out
}// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// PERSISTÃƒÅ NCIA DE SESSÃƒÆ’O (TXT Ã¢â‚¬â€ pulados)
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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
        .expect("Erro ao abrir arquivo de sessÃƒÂ£o de pulados");
    writeln!(file, "{}", id).expect("Erro ao salvar ID pulado na sessÃƒÂ£o");
}

fn reescrever_pulados_sessao(pulados: &HashSet<String>) {
    let conteudo: String = pulados.iter().cloned().collect::<Vec<_>>().join("\n");
    let com_newline = if conteudo.is_empty() {
        String::new()
    } else {
        format!("{}\n", conteudo)
    };
    fs::write(SESSION_FILE, com_newline)
        .unwrap_or_else(|e| eprintln!("[AVISO] Erro ao reescrever sessÃƒÂ£o: {}", e));
}

fn limpar_sessao() {
    let _ = fs::remove_file(SESSION_FILE);
    let _ = fs::remove_file(SESSION_STATE_FILE);
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// PERSISTÃƒÅ NCIA DE ESTADO COMPLETO (JSON Ã¢â‚¬â€ aprovados, rejeitados, etc.)
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

fn salvar_estado_sessao(estado: &EstadoSessao) {
    let ultimo = match &estado.ultimo_documento_id {
        Some(id) => format!("\"{}\"", id),
        None => "null".to_string(),
    };
    let json = format!(
        "{{\n  \"started_at\": \"{}\",\n  \"aprovados\": {},\n  \"rejeitados\": {},\n  \"pulados\": {},\n  \"ultimo_documento_id\": {}\n}}\n",
        estado.started_at, estado.aprovados, estado.rejeitados, estado.pulados, ultimo
    );
    fs::write(SESSION_STATE_FILE, json)
        .unwrap_or_else(|e| eprintln!("[AVISO] Erro ao salvar estado da sessÃƒÂ£o: {}", e));
}

/// Extrai o valor de uma chave no JSON simples gerado por `salvar_estado_sessao`.
/// Retorna None se a chave nÃƒÂ£o existir ou o valor for `null`.
fn extrair_json_valor<'a>(conteudo: &'a str, chave: &str) -> Option<&'a str> {
    let prefixo = format!("\"{}\":", chave);
    for linha in conteudo.lines() {
        let linha = linha.trim();
        if linha.starts_with(&prefixo) {
            let resto = linha[prefixo.len()..].trim().trim_end_matches(',');
            if resto == "null" {
                return None;
            }
            if resto.starts_with('"') && resto.ends_with('"') {
                return Some(&resto[1..resto.len() - 1]);
            }
            return Some(resto);
        }
    }
    None
}

fn carregar_estado_sessao() -> Option<EstadoSessao> {
    let conteudo = fs::read_to_string(SESSION_STATE_FILE).ok()?;
    let started_at = extrair_json_valor(&conteudo, "started_at")?.to_string();
    let aprovados: u32 = extrair_json_valor(&conteudo, "aprovados")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let rejeitados: u32 = extrair_json_valor(&conteudo, "rejeitados")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let pulados: u32 = extrair_json_valor(&conteudo, "pulados")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let ultimo_documento_id =
        extrair_json_valor(&conteudo, "ultimo_documento_id").map(|s| s.to_string());

    Some(EstadoSessao {
        started_at,
        aprovados,
        rejeitados,
        pulados,
        ultimo_documento_id,
    })
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// BANCO DE DADOS
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

fn contar_pendentes(client: &mut Client, pulados: &HashSet<String>) -> i64 {
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

/// Busca o lote de documentos.
/// O preview ÃƒÂ© extraÃƒÂ­do de atÃƒÂ© 4000 chars do conteÃƒÂºdo e entÃƒÂ£o filtrado via
/// `filtrar_preview`, que descarta linhas curtas e duplicadas.
fn buscar_lote(
    client: &mut Client,
    pulados_sessao: &HashSet<String>,
    batch_size: i64,
) -> Vec<Documento> {
    let excluidos: Vec<&str> = pulados_sessao.iter().map(|s| s.as_str()).collect();

    let rows = if excluidos.is_empty() {
        client.query(
            "SELECT d.id::text, d.source, d.domain, d.doc_type, d.content_length,
                    LEFT(convert_from(convert_to(d.content,'UTF8'),'UTF8'), 4000),
                    RIGHT(convert_from(convert_to(d.content,'UTF8'),'UTF8'), 1000)
             FROM documents d JOIN validation v ON v.document_id = d.id
             WHERE v.status = 'pending'
             ORDER BY d.domain, d.collected_at LIMIT $1",
            &[&batch_size],
        )
    } else {
        client.query(
            "SELECT d.id::text, d.source, d.domain, d.doc_type, d.content_length,
                    LEFT(convert_from(convert_to(d.content,'UTF8'),'UTF8'), 4000),
                    RIGHT(convert_from(convert_to(d.content,'UTF8'),'UTF8'), 1000)
             FROM documents d JOIN validation v ON v.document_id = d.id
             WHERE v.status = 'pending' AND d.id::text != ALL($1)
             ORDER BY d.domain, d.collected_at LIMIT $2",
            &[&excluidos, &batch_size],
        )
    }
    .expect("Erro ao buscar documentos");

    rows.iter()
        .map(|row| {
            let raw: String = row.get(5);
            let tail: String = row.get(6);
            let head: String = raw.chars().take(1000).collect();
            let preview = filtrar_preview(&raw);
            Documento {
                id: row.get(0),
                source: row.get(1),
                domain: row.get(2),
                doc_type: row.get(3),
                content_length: row.get(4),
                preview,
                content: tail,
                head,
            }
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
        .expect("Erro ao buscar conteÃƒÂºdo completo");
    row.get::<_, String>(0)
}

fn db_aprovar(client: &mut Client, id: &str) {
    client
        .execute(
            "UPDATE validation SET status = 'approved', decided_by = 'human', \
             decided_at = NOW() WHERE document_id::text = $1",
            &[&id],
        )
        .expect("Erro ao aprovar documento");
}

fn db_rejeitar(client: &mut Client, id: &str, motivo: &str) {
    client
        .execute(
            "UPDATE validation SET status = 'rejected', rejection_reason = $2, \
             decided_by = 'human', decided_at = NOW() \
             WHERE document_id::text = $1",
            &[&id, &motivo],
        )
        .expect("Erro ao rejeitar documento");
}

fn db_desfazer(client: &mut Client, id: &str) {
    client
        .execute(
            "UPDATE validation SET status = 'pending', decided_by = 'pending', \
             decided_at = NULL, rejection_reason = NULL \
             WHERE document_id::text = $1",
            &[&id],
        )
        .expect("Erro ao desfazer decisÃƒÂ£o");
}

fn contar_por_dominio(client: &mut Client) -> Vec<(String, i64, i64, i64)> {
    let rows = client
        .query(
            "SELECT d.domain,
                    COUNT(*) FILTER (WHERE v.status = 'pending')   AS pending,
                    COUNT(*) FILTER (WHERE v.status = 'approved')  AS approved,
                    COUNT(*) FILTER (WHERE v.status = 'rejected')  AS rejected
             FROM documents d
             JOIN validation v ON v.document_id = d.id
             GROUP BY d.domain
             ORDER BY d.domain",
            &[],
        )
        .expect("Erro ao contar por domÃƒÂ­nio");
    rows.iter()
        .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3)))
        .collect()
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// FILTRO DE PREVIEW
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Recebe o conteÃƒÂºdo bruto (atÃƒÂ© 4 000 chars) e retorna atÃƒÂ© 800 chars de
/// conteÃƒÂºdo de qualidade:
/// - Pula linhas com menos de 30 chars (links, itens de menu, etc.)
/// - Pula linhas duplicadas
/// - Acumula atÃƒÂ© 800 chars (contando codepoints para consistÃƒÂªncia com a TUI)
///
/// Fallback: se o filtro produzir menos de 100 chars, retorna os primeiros
/// 800 chars sem filtro (documentos muito curtos ou todo em linhas pequenas).
fn filtrar_preview(conteudo: &str) -> String {
    let mut resultado = String::new();
    let mut visto: HashSet<&str> = HashSet::new();

    for linha in conteudo.lines() {
        let l = linha.trim();
        if l.chars().count() < 30 {
            continue;
        }
        if !visto.insert(l) {
            continue;
        }
        resultado.push_str(l);
        resultado.push('\n');
        if resultado.chars().count() >= 800 {
            break;
        }
    }

    if resultado.trim().chars().count() < 100 {
        // Fallback: sem filtro, apenas trunca por codepoints
        conteudo.chars().take(800).collect()
    } else {
        resultado
    }
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// INTERFACE Ã¢â‚¬â€ UTILIDADES
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

fn limpar_tela() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().expect("Erro ao limpar tela");
}

fn ler_linha(stdin: &io::Stdin) -> String {
    let mut buf = String::new();
    stdin
        .lock()
        .read_line(&mut buf)
        .expect("Erro ao ler linha do stdin");
    buf.trim().to_lowercase()
}

/// Conta codepoints Unicode Ã¢â‚¬â€ nÃƒÂ£o bytes. Resolve o desalinhamento causado por
/// sÃƒÂ­mbolos multi-byte (Ã¢Å“â€œ Ã¢Å“â€” ~) no cÃƒÂ¡lculo de padding do header.
/// Nota: nÃƒÂ£o ÃƒÂ© perfeito para caracteres CJK de largura dupla, mas cobre
/// inteiramente os sÃƒÂ­mbolos usados neste programa.
fn largura_visual(s: &str) -> usize {
    s.chars().count()
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// INTERFACE Ã¢â‚¬â€ EXIBIÃƒâ€¡ÃƒÆ’O
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

fn exibir_ajuda() {
    println!();
    println!("  Ã¢â€Å’Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â");
    println!("  Ã¢â€â€š  COMANDOS                                        Ã¢â€â€š");
    println!("  Ã¢â€â€š                                                  Ã¢â€â€š");
    println!("  Ã¢â€â€š  [a] Aprovar                                     Ã¢â€â€š");
    println!("  Ã¢â€â€š  [r] Rejeitar  (pede motivo)                     Ã¢â€â€š");
    println!("  Ã¢â€â€š  [u] InÃƒÂºtil    (rejeiÃƒÂ§ÃƒÂ£o rÃƒÂ¡pida, sem digitar)    Ã¢â€â€š");
    println!("  Ã¢â€â€š  [p] Pular     (deixa como pendente, lembra)     Ã¢â€â€š");
    println!("  Ã¢â€â€š  [v] Voltar    (desfaz ÃƒÂºltima decisÃƒÂ£o)           Ã¢â€â€š");
    println!("  Ã¢â€â€š  [?] Ver mais  (exibe conteÃƒÂºdo completo paginado)Ã¢â€â€š");
    println!("  Ã¢â€â€š  [i] SugestÃƒÂ£o  (anÃƒÂ¡lise do documento atual)      Ã¢â€â€š");
    println!("  Ã¢â€â€š  [h] HeurÃƒÂ­stica (on/off sugestÃƒÂ£o heurÃƒÂ­stica)     Ã¢â€â€š");
    println!("  Ã¢â€â€š  [t] Auto-IA   (liga/desliga IA automÃƒÂ¡tica)      Ã¢â€â€š");
    println!("  Ã¢â€â€š  [e] EstatÃƒÂ­sticas da sessÃƒÂ£o                      Ã¢â€â€š");
    println!("  Ã¢â€â€š  [s] Salvar e sair                               Ã¢â€â€š");
    println!("  Ã¢â€â€š  [q] Sair      (descarta pulados da sessÃƒÂ£o)      Ã¢â€â€š");
    println!("  Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ");
    println!();
}

#[allow(clippy::too_many_arguments)] // A renderização da TUI mantém parâmetros explícitos.
fn exibir_documento(
    doc: &Documento,
    idx: usize,
    total_lote: usize,
    aprovados: u32,
    rejeitados: u32,
    pulados_count: u32,
    total_pendentes: i64,
    linhas: usize,
    modo: &ModoSugestao,
) {
    limpar_tela();

    let progresso_total = aprovados + rejeitados;
    let total_geral = progresso_total as i64 + total_pendentes;
    let conteudo_header = format!(
        "  Lote: {}/{}   SessÃƒÂ£o: Ã¢Å“â€œ{}  Ã¢Å“â€”{}  ~{}   Total: {}/{}   Modo: {}",
        idx + 1,
        total_lote,
        aprovados,
        rejeitados,
        pulados_count,
        progresso_total,
        total_geral,
        modo.label()
    );
    // Usa largura_visual para nÃƒÂ£o contar bytes dos sÃƒÂ­mbolos Unicode como colunas
    let padding = 62usize.saturating_sub(largura_visual(&conteudo_header));

    println!("Ã¢â€¢â€Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢â€”");
    println!("Ã¢â€¢â€˜  NEXUS Ã¢â‚¬â€ VALIDADOR DE DOCUMENTOS                             Ã¢â€¢â€˜");
    println!("Ã¢â€¢Â Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â£");
    println!("Ã¢â€¢â€˜{}{}Ã¢â€¢â€˜", conteudo_header, " ".repeat(padding));
    println!("Ã¢â€¢Å¡Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â");
    println!();
    println!("  DomÃƒÂ­nio  : {}", doc.domain);
    println!("  Tipo     : {}", doc.doc_type);
    println!("  Tamanho  : {} bytes", doc.content_length);
    println!("  Fonte    : {}", doc.source);
    println!();
    println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
    println!("  PRÃƒâ€°VIA DO CONTEÃƒÅ¡DO ({} linhas):", linhas);
    println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
    println!();

    for linha in doc.preview.lines().take(linhas) {
        println!("  {}", linha);
    }

    println!();
    println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
    println!(
        "  [a] Aprovar  [r] Rejeitar  [u] InÃƒÂºtil  [p] Pular  [b] Browser  [s] Salvar  [q] Sair"
    );
    println!(
        "  [i] SugestÃƒÂ£o [h] HeurÃƒÂ­stica  [t] Auto-IA  [x] Parar-IA  [v] Voltar  [?] Ver mais  [e] Stats  [z] Config"
    );
    println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
}

fn quebrar_motivo(texto: &str, largura: usize) -> Vec<String> {
    let mut linhas: Vec<String> = Vec::new();
    let mut linha_atual = String::new();
    for palavra in texto.split_whitespace() {
        if !linha_atual.is_empty()
            && largura_visual(&linha_atual) + 1 + largura_visual(palavra) > largura
        {
            linhas.push(linha_atual.clone());
            linha_atual.clear();
        }
        if !linha_atual.is_empty() {
            linha_atual.push(' ');
        }
        linha_atual.push_str(palavra);
    }
    if !linha_atual.is_empty() {
        linhas.push(linha_atual);
    }
    linhas
}

fn exibir_sugestao(s: &Sugestao) {
    println!("{}", formatar_sugestao(s));
}

fn exibir_estatisticas(
    client: &mut Client,
    inicio_sessao: std::time::Instant,
    aprovados: u32,
    rejeitados: u32,
    stdin: &io::Stdin,
) {
    let dominios = contar_por_dominio(client);
    let decorrido = inicio_sessao.elapsed().as_secs();
    let total_decididos = (aprovados + rejeitados) as f64;
    let horas = decorrido as f64 / 3600.0;
    let velocidade = if horas > 0.01 {
        total_decididos / horas
    } else {
        0.0
    };
    let total_pending: i64 = dominios.iter().map(|(_, p, _, _)| p).sum();
    let restante_horas = if velocidade > 0.0 {
        total_pending as f64 / velocidade
    } else {
        0.0
    };
    let h = decorrido / 3600;
    let m = (decorrido % 3600) / 60;
    let s_sec = decorrido % 60;

    println!();
    println!("  Ã¢â€Å’Ã¢â€â‚¬ ESTATÃƒÂSTICAS DA SESSÃƒÆ’O Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â");
    println!(
        "  Ã¢â€â€š  DuraÃƒÂ§ÃƒÂ£o:    {:02}:{:02}:{:02}                              Ã¢â€â€š",
        h, m, s_sec
    );
    println!(
        "  Ã¢â€â€š  Velocidade: {:.1} docs/hora                         Ã¢â€â€š",
        velocidade
    );
    println!(
        "  Ã¢â€â€š  Restante:   ~{:.0}h (baseado em {} pending)        Ã¢â€â€š",
        restante_horas, total_pending
    );
    println!("  Ã¢â€â€š                                                      Ã¢â€â€š");
    println!("  Ã¢â€â€š  DOMÃƒÂNIO    PENDING  APROVADO  REJEITADO            Ã¢â€â€š");
    for (dom, pend, aprov, rejeit) in &dominios {
        println!(
            "  Ã¢â€â€š  {:<10} {:>6}   {:>7}   {:>8}            Ã¢â€â€š",
            dom, pend, aprov, rejeit
        );
    }
    println!("  Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ");
    println!();
    print!("  [Enter] para fechar: ");
    io::stdout()
        .flush()
        .expect("Erro ao exibir prompt de fechar estatÃƒÂ­sticas");
    let mut buf = String::new();
    stdin
        .lock()
        .read_line(&mut buf)
        .expect("Erro ao ler confirmaÃƒÂ§ÃƒÂ£o de estatÃƒÂ­sticas");
}

fn exibir_conteudo_completo(conteudo: &str, source: &str, stdin: &io::Stdin) {
    let linhas_conteudo: Vec<&str> = conteudo.lines().collect();
    let total_linhas = linhas_conteudo.len();
    let pagina_size = 40usize;
    let mut offset = 0usize;

    loop {
        limpar_tela();
        let fim = (offset + pagina_size).min(total_linhas);
        println!(
            "Ã¢â€¢ÂÃ¢â€¢Â {} | Linhas {}-{} de {} | [Enter] prÃƒÂ³x  [b] ant  [q] sair Ã¢â€¢ÂÃ¢â€¢Â",
            source,
            offset + 1,
            fim,
            total_linhas
        );
        println!();
        for linha in &linhas_conteudo[offset..fim] {
            println!("  {}", linha);
        }
        println!();
        print!("  Comando: ");
        io::stdout()
            .flush()
            .expect("Erro ao exibir prompt do paginador");
        let cmd = ler_linha(stdin);
        match cmd.as_str() {
            "b" => {
                offset = offset.saturating_sub(pagina_size);
            }
            "q" => break,
            "" => {
                if fim < total_linhas {
                    offset = fim;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
}

fn exibir_conteudo_completo_rx(
    conteudo: &str,
    source: &str,
    rx: &std::sync::mpsc::Receiver<String>,
) {
    let linhas_conteudo: Vec<&str> = conteudo.lines().collect();
    let total_linhas = linhas_conteudo.len();
    let pagina_size = 40usize;
    let mut offset = 0usize;
    loop {
        let fim = (offset + pagina_size).min(total_linhas);
        println!();
        println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
        println!(
            "  {} | Linhas {}-{} de {} | [Enter] prÃƒÂ³x  [b] ant  [q] sair",
            source,
            offset + 1,
            fim,
            total_linhas
        );
        println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
        for linha in &linhas_conteudo[offset..fim] {
            println!("  {}", linha);
        }
        println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
        print!("  Comando: ");
        io::stdout().flush().expect("flush paginador_rx");
        let cmd = rx
            .recv_timeout(std::time::Duration::from_secs(120))
            .unwrap_or_default();
        match cmd.as_str() {
            "b" => {
                offset = offset.saturating_sub(pagina_size);
            }
            "q" => break,
            "" => {
                if fim < total_linhas {
                    offset = fim;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
}
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// ALGORITMO DE SUGESTÃƒÆ’O
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

fn db_aprovar_ia(client: &mut Client, id: &str) {
    client.execute(
        "UPDATE validation SET status = 'approved', decided_by = 'ai',          decided_at = NOW() WHERE document_id::text = $1",
        &[&id],
    ).unwrap_or_else(|e| { eprintln!("[ERRO] db_aprovar_ia: {}", e); 0 });
}

fn db_rejeitar_ia(client: &mut Client, id: &str, motivo: &str) {
    client.execute(
        "UPDATE validation SET status = 'rejected', decided_by = 'ai',          rejection_reason = $2, decided_at = NOW() WHERE document_id::text = $1",
        &[&id, &motivo],
    ).unwrap_or_else(|e| { eprintln!("[ERRO] db_rejeitar_ia: {}", e); 0 });
}

const OLLAMA_URL: &str = "http://localhost:11434/api/generate";
const CONFIANCA_MINIMA: u8 = 60;

fn validar_resposta(motivo: &str, confianca: u8) -> bool {
    if confianca < CONFIANCA_MINIMA {
        return false;
    }
    if motivo.len() < 10 {
        return false;
    }
    true
}

fn chamar_ollama(dominio: &str, conteudo: &str, tentativa: u8, timeout_s: u64) -> Option<Sugestao> {
    let instrucao_extra = if tentativa > 1 {
        " IMPORTANTE: Seu motivo DEVE mencionar termos tecnicos especificos do dominio."
    } else {
        ""
    };

    let trecho = &conteudo[..conteudo.len().min(1500)];
    let prompt = format!(
        "[INST] Voce e um classificador tecnico rigoroso.{extra}\n\nDominio alvo: {dom}\n\nDocumento:\n{doc}\n\nEste documento tem profundidade tecnica util para treinar IA especializada em {dom}?\nutil=true: explica conceitos, APIs, implementacoes ou comportamentos tecnicos especificos.\nutil=false: e apenas navegacao, lista de siglas, changelog, ou conteudo raso sem explicacao.\n\nDescreva em uma frase O QUE o documento contem.\nResponda em PORTUGUES somente com JSON sem markdown:\n{{\"util\": true_ou_false, \"confianca\": numero_0_a_100, \"motivo\": \"uma_frase_descrevendo_o_conteudo\"}} [/INST]",
        extra = instrucao_extra,
        dom = dominio,
        doc = trecho,
    );

    let body = serde_json::json!({
        "model": "mistral",
        "prompt": prompt,
        "stream": false,
        "options": {"temperature": 0.1, "num_predict": 150}
    });

    let resp = ureq::post(OLLAMA_URL)
        .timeout(std::time::Duration::from_secs(timeout_s))
        .send_json(body)
        .ok()?;

    let json: serde_json::Value = resp.into_json().ok()?;
    let texto = json["response"].as_str()?.trim().to_string();

    let inicio = texto.find('{')?;
    let fim = texto.rfind('}')? + 1;
    let parsed: serde_json::Value = serde_json::from_str(&texto[inicio..fim]).ok()?;

    let util = parsed["util"].as_bool()?;
    let confianca = parsed["confianca"].as_u64().unwrap_or(0) as u8;
    let motivo = parsed["motivo"].as_str().unwrap_or("").to_string();

    Some(Sugestao {
        categoria: if util {
            Categoria::Util
        } else {
            Categoria::Inutil
        },
        confianca,
        motivo,
    })
}

fn sugerir_ia(dominio: &str, conteudo: &str, timeout_s: u64) -> Option<Sugestao> {
    for tentativa in 1u8..=2 {
        if let Some(s) = chamar_ollama(dominio, conteudo, tentativa, timeout_s) {
            if validar_resposta(&s.motivo, s.confianca) {
                return Some(s);
            }
            if tentativa == 2 {
                return Some(Sugestao {
                    confianca: s.confianca / 2,
                    motivo: format!("[suspeito] {}", s.motivo),
                    ..s
                });
            }
        }
    }
    None
}

fn sugerir_com_ia(doc: &Documento) -> Sugestao {
    if let Some(provider) = SugestorSocketProvider::from_env()
        && let Ok(s) = obter_sugestao(&provider, doc)
    {
        return s;
    }
    sugerir_ia(&doc.domain, &doc.content, 30).unwrap_or_else(|| sugerir_heuristica_interna(doc))
}

fn sugerir_heuristica(doc: &Documento) -> Sugestao {
    sugerir_heuristica_interna(doc)
}

fn sugerir_heuristica_interna(doc: &Documento) -> Sugestao {
    let texto = doc.preview.to_lowercase();
    let linhas: Vec<&str> = doc
        .preview
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let total = linhas.len().max(1);
    let total_palavras = texto.split_whitespace().count().max(1);

    let mut pontos_ruins: i32 = 0;
    let mut motivos_ruins: Vec<String> = Vec::new();

    // Ã¢â€â‚¬Ã¢â€â‚¬ Assinaturas eliminatÃƒÂ³rias Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let assinaturas_fortes: &[(&str, &str)] = &[
        ("table of contents\n", "pÃƒÂ¡gina de TOC puro"),
        ("your account\ndocumentation\n", "cabeÃƒÂ§alho postgresql.org"),
        ("next page\nprevious page\n", "paginaÃƒÂ§ÃƒÂ£o de navegaÃƒÂ§ÃƒÂ£o"),
        ("skip to main content\n", "pÃƒÂ¡gina sem conteÃƒÂºdo real"),
        ("search results for", "pÃƒÂ¡gina de resultado de busca"),
    ];
    for (assinatura, descricao) in assinaturas_fortes {
        if texto.contains(assinatura) {
            return Sugestao {
                categoria: Categoria::Inutil,
                confianca: 95,
                motivo: format!("EliminatÃƒÂ³rio: {}", descricao),
            };
        }
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ CabeÃƒÂ§alho de idioma Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let tem_cabecalho_idioma =
        texto.contains("chinese (simplified)") || texto.contains("chinese (traditional)");
    if tem_cabecalho_idioma {
        pontos_ruins += 15;
        motivos_ruins.push("cabeÃƒÂ§alho de idioma presente".to_string());
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Linhas curtas Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let padroes_codigo: &[&str] = &[
        "=", "::", "->", "__", "{}", "()", "[]", "#", "/*", "*/", "//",
    ];
    let curtas = linhas
        .iter()
        .filter(|l| {
            if l.len() >= 20 {
                return false;
            }
            !padroes_codigo.iter().any(|p| l.contains(p))
        })
        .count();
    let pct_curtas = (curtas * 100) / total;
    if pct_curtas > 75 {
        pontos_ruins += 40;
        motivos_ruins.push(format!("{}% das linhas < 20 chars", pct_curtas));
    } else if pct_curtas > 55 {
        pontos_ruins += 20;
        motivos_ruins.push(format!("{}% das linhas sÃƒÂ£o curtas", pct_curtas));
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Duplicatas Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // Distingue duplicatas de navegaÃƒÂ§ÃƒÂ£o (linhas isoladas repetidas, ex: menus)
    // de artefatos de scraping (blocos consecutivos repetidos, ex: cÃƒÂ³digo renderizado 2x).
    // Apenas duplicatas isoladas sÃƒÂ£o penalizadas Ã¢â‚¬â€ blocos sÃƒÂ£o ignorados.
    let mut vistos: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut duplicadas_isoladas = 0usize;
    let mut i_dup = 0usize;
    while i_dup < linhas.len() {
        let l = linhas[i_dup];
        if l.len() > 5 && !vistos.insert(l) {
            // Verifica se faz parte de um bloco consecutivo duplicado (2+ linhas seguidas)
            let prev_dup = i_dup > 0 && {
                let lp = linhas[i_dup - 1];
                lp.len() > 5 && !vistos.contains(lp)
            };
            let next_dup = i_dup + 1 < linhas.len() && {
                let ln = linhas[i_dup + 1];
                ln.len() > 5 && vistos.contains(ln)
            };
            // SÃƒÂ³ penaliza se for duplicata isolada (nÃƒÂ£o parte de bloco)
            if !prev_dup && !next_dup {
                duplicadas_isoladas += 1;
            }
        }
        i_dup += 1;
    }
    let pct_dup = (duplicadas_isoladas * 100) / total;
    if pct_dup > 50 {
        pontos_ruins += 35;
        motivos_ruins.push(format!("{}% das linhas duplicadas (navegacao)", pct_dup));
    } else if pct_dup > 30 {
        pontos_ruins += 15;
        motivos_ruins.push(format!("{}% de duplicatas de navegacao", pct_dup));
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ DetecÃƒÂ§ÃƒÂ£o de TOC/ÃƒÂ­ndice Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let linhas_lista = linhas
        .iter()
        .filter(|l| {
            let l_low = l.to_lowercase();
            l.ends_with(|c: char| c.is_ascii_digit())
                || l.starts_with("- ")
                || l.starts_with("* ")
                || l.starts_with("Ã¢â‚¬Â¢ ")
                || (l_low == **l
                    && !l.ends_with('.')
                    && !l.ends_with('?')
                    && !l.ends_with('!')
                    && l.split_whitespace().count() <= 6)
        })
        .count();
    let pct_lista = (linhas_lista * 100) / total;
    if pct_lista > 40 && total > 15 {
        pontos_ruins += 30;
        motivos_ruins.push(format!(
            "{}% das linhas sÃƒÂ£o itens de lista/ÃƒÂ­ndice",
            pct_lista
        ));
    }

    // MELHORIA 1: TOC com padrÃƒÂ£o numÃƒÂ©rico inline Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let mut linhas_toc_inline = 0usize;
    for linha in &linhas {
        let chars: Vec<char> = linha.chars().collect();
        let len = chars.len();
        let mut ocorrencias = 0usize;
        let mut i = 0usize;
        while i < len {
            if chars[i].is_ascii_digit() {
                let mut j = i;
                while j < len && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j < len && chars[j] == '.' {
                    j += 1;
                    if j < len && chars[j].is_ascii_digit() {
                        while j < len && chars[j].is_ascii_digit() {
                            j += 1;
                        }
                        if j < len && chars[j] == '.' {
                            ocorrencias += 1;
                            i = j + 1;
                            continue;
                        }
                    }
                }
            }
            i += 1;
        }
        if ocorrencias >= 3 {
            linhas_toc_inline += 1;
        }
    }
    let pct_toc_inline = (linhas_toc_inline * 100) / total;

    // Detecta ÃƒÂ­ndice de capÃƒÂ­tulo estilo "35.3. nome" ou "35.3 nome" (postgresql, etc.)
    let linhas_chapter_idx = linhas
        .iter()
        .filter(|l| {
            let b = l.as_bytes();
            let mut i = 0;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i == 0 || i >= b.len() || b[i] != b'.' {
                return false;
            }
            i += 1;
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            i > start // tinha dÃƒÂ­gitos apÃƒÂ³s o ponto
        })
        .count();
    let pct_chapter_idx = (linhas_chapter_idx * 100) / total;
    if pct_chapter_idx > 30 && total > 5 {
        pontos_ruins += 50;
        motivos_ruins.push(format!(
            "{}% das linhas com padrÃƒÂ£o de ÃƒÂ­ndice de capÃƒÂ­tulo (N.N.)",
            pct_chapter_idx
        ));
    }

    let sinal_toc_ativo = pct_lista > 40 || pct_toc_inline > 20 || pct_chapter_idx > 30;
    if pct_toc_inline > 20 {
        pontos_ruins += 40;
        motivos_ruins.push(format!(
            "{}% das linhas com padrÃƒÂ£o numÃƒÂ©rico de ÃƒÂ­ndice",
            pct_toc_inline
        ));
    }

    // MELHORIA 3: idioma + TOC Ã¢â€ â€™ eliminatÃƒÂ³rio Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    if tem_cabecalho_idioma && sinal_toc_ativo {
        return Sugestao {
            categoria: Categoria::Inutil,
            confianca: 92,
            motivo: "ÃƒÂ­ndice kernel.org com cabeÃƒÂ§alho de idioma".to_string(),
        };
    }

    // MELHORIA 2: RazÃƒÂ£o tÃƒÂ­tulo/conteÃƒÂºdo Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let linhas_longas = linhas.iter().filter(|l| l.len() > 100).count();
    let padroes_api_pre: &[&str] = &["pub fn", "pub struct", "pub trait", "impl "];
    let linhas_api_pre = linhas
        .iter()
        .filter(|l| padroes_api_pre.iter().any(|p| l.contains(p)))
        .count();
    let sinal_api_ref = !sinal_toc_ativo && linhas_api_pre >= 5;
    let linhas_curtas_medias = linhas.iter().filter(|l| l.len() < 60).count();
    if linhas_longas < 5 && linhas_curtas_medias > 20 && !sinal_api_ref {
        pontos_ruins += 30;
        motivos_ruins.push("estrutura de ÃƒÂ­ndice: poucos parÃƒÂ¡grafos, muitos tÃƒÂ­tulos".to_string());
    }
    if pct_dup > 20 && linhas_longas < 3 && !sinal_api_ref {
        pontos_ruins += 15;
        motivos_ruins.push("estrutura repetitiva com poucas linhas longas".to_string());
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Releases/downloads Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let palavras_release = [
        "download",
        "release",
        "changelog",
        "version ",
        "v0.",
        "v1.",
        "v2.",
    ];
    let linhas_release = linhas
        .iter()
        .filter(|l| palavras_release.iter().any(|p| l.contains(p)))
        .count();
    let pct_release = (linhas_release * 100) / total;
    if pct_release > 30 {
        pontos_ruins += 20;
        motivos_ruins.push("possÃƒÂ­vel pÃƒÂ¡gina de releases/downloads".to_string());
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Tamanho Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    if doc.content_length < 800 {
        pontos_ruins += 40;
        motivos_ruins.push(format!("apenas {} bytes", doc.content_length));
    } else if doc.content_length < 1500 {
        pontos_ruins += 10;
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ PontuaÃƒÂ§ÃƒÂ£o Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let chars_total = texto.len().max(1);
    let pontuacao = texto
        .chars()
        .filter(|&c| c == '.' || c == ',' || c == ';' || c == ':')
        .count();
    let pct_pont = (pontuacao * 1000) / chars_total;
    if pct_pont < 2 && total > 20 {
        pontos_ruins += 25;
        motivos_ruins.push("quase sem pontuaÃƒÂ§ÃƒÂ£o".to_string());
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Palavras-chave por domÃƒÂ­nio Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let palavras_rust: &[&str] = &[
        "fn ",
        "let ",
        "mut ",
        "impl ",
        "trait ",
        "struct ",
        "enum ",
        "match ",
        "borrow",
        "lifetime",
        "ownership",
        "unsafe",
        "cargo",
        "crate",
        "async",
        "await",
        "result",
        "option",
        "closure",
        "iterator",
        "generic",
        "compiler",
        "rustc",
        "macro",
        "pattern",
        "reference",
        "slice",
    ];
    let palavras_infra: &[&str] = &[
        "kernel",
        "syscall",
        "driver",
        "interrupt",
        "memory management",
        "process",
        "thread",
        "scheduler",
        "filesystem",
        "network stack",
        "docker",
        "container",
        "volume",
        "postgres",
        "sql",
        "query",
        "index",
        "systemd",
        "service unit",
        "socket",
        "journal",
        "cgroup",
        "namespace",
        "ioctl",
        "mmap",
        "fork",
        "signal",
        "buffer",
        "mount",
        "inode",
        "lock",
        "mutex",
        "semaphore",
        "spinlock",
        "rcu",
        "dma",
        "framebuffer",
        "platform_device",
        "pci",
        "vga",
        "errno",
        "probe",
        "unregister",
        "register",
        "irq",
        "mmio",
        "aperture",
        "resource_size",
        "firmware",
        "bootloader",
        "hypervisor",
    ];
    let palavras_security: &[&str] = &[
        "vulnerability",
        "cve-",
        "exploit",
        "injection",
        "xss",
        "csrf",
        "authentication",
        "authorization",
        "encryption",
        "tls",
        "ssl",
        "owasp",
        "nist",
        "rfc ",
        "attack vector",
        "mitigation",
        "privilege escalation",
        "sanitize",
        "certificate",
        "hash",
        "buffer overflow",
        "memory corruption",
        "denial of service",
    ];
    let palavras_mlops: &[&str] = &[
        "fine-tuning",
        "lora",
        "qlora",
        "transformer",
        "attention mechanism",
        "embedding",
        "dataset",
        "gradient",
        "optimizer",
        "loss function",
        "huggingface",
        "pytorch",
        "llm",
        "tokenizer",
        "quantization",
        "inference",
        "training loop",
        "batch size",
        "learning rate",
        "peft",
        "adapter",
        "checkpoint",
        "model weights",
    ];
    let (palavras, nome): (&[&str], &str) = match doc.domain.as_str() {
        "rust" => (palavras_rust, "Rust"),
        "infra" => (palavras_infra, "Infra"),
        "security" => (palavras_security, "Security"),
        "mlops" => (palavras_mlops, "MLOps"),
        _ => (palavras_infra, "geral"),
    };

    let texto_meio = doc.head.to_lowercase();
    let texto_fim = doc.content.to_lowercase();
    let hits: Vec<&str> = palavras
        .iter()
        .filter(|p| texto.contains(**p) || texto_meio.contains(**p) || texto_fim.contains(**p))
        .map(|p| p.trim())
        .collect();
    let n = hits.len();

    // Ã¢â€â‚¬Ã¢â€â‚¬ BÃƒÂ´nus Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let densidade = (n * 100) / total_palavras;
    let bonus_densidade: i32 = if densidade > 8 {
        25
    } else if densidade > 4 {
        10
    } else {
        0
    };
    let bonus_tecnico_bruto: i32 = (if n >= 8 {
        50
    } else if n >= 5 {
        35
    } else if n >= 3 {
        20
    } else if n >= 1 {
        8
    } else {
        0
    }) + bonus_densidade;
    let bonus_tecnico_base: i32 = if pct_curtas > 60 {
        bonus_tecnico_bruto / 3
    } else if pct_curtas > 40 {
        bonus_tecnico_bruto / 2
    } else {
        bonus_tecnico_bruto
    };

    let source_lower = doc.source.to_lowercase();
    let padroes_url_tecnicos: &[&str] = &[
        "kernel.org/doc",
        "/dev-tools/",
        "/core-api/",
        "/driver-api/",
        "/locking/",
        "/process/submitting",
        "/rust/",
        "/nomicon",
        "/reference",
        "docs.rs/",
        "/rfc",
        "owasp.org",
        "nist.gov",
        "huggingface.co/docs",
        "arxiv.org",
    ];
    let padroes_url_lixo: &[&str] = &["/_sources/", "/genindex", "/search", "/translations/"];
    let bonus_url: i32 = if padroes_url_lixo.iter().any(|p| source_lower.contains(p)) {
        -15
    } else if padroes_url_tecnicos
        .iter()
        .any(|p| source_lower.contains(p))
    {
        20
    } else {
        0
    };
    if bonus_url < 0 {
        pontos_ruins += -bonus_url;
        motivos_ruins.push("URL indica conteÃƒÂºdo de navegaÃƒÂ§ÃƒÂ£o/ÃƒÂ­ndice".to_string());
    }

    let bonus_tamanho: i32 = if sinal_toc_ativo || pct_lista > 60 {
        0
    } else if doc.content_length >= 20_000 {
        35
    } else if doc.content_length >= 10_000 {
        25
    } else if doc.content_length >= 5_000 {
        15
    } else {
        0
    };
    let bonus_api_ref: i32 = if sinal_api_ref { 20 } else { 0 };
    let bonus_tecnico = bonus_tecnico_base
        + if bonus_url > 0 { bonus_url } else { 0 }
        + bonus_tamanho
        + bonus_api_ref;
    let score_final = pontos_ruins - bonus_tecnico;

    // Ã¢â€â‚¬Ã¢â€â‚¬ DecisÃƒÂ£o Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    if n >= 3 && score_final < 30 {
        let exemplos = hits
            .iter()
            .take(4)
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", ");
        let bonus_neg: i32 = if score_final < 0 {
            (-score_final / 5).min(20)
        } else {
            0
        };
        let confianca = ((55 + n * 4) as i32 - score_final.max(0) + bonus_neg).clamp(30, 95) as u8;
        let mut sufixo = String::new();
        if score_final < 0 {
            sufixo.push_str(&format!(" [score: {}]", score_final));
        }
        if bonus_tamanho > 0 {
            sufixo.push_str(&format!(" [{} bytes]", doc.content_length));
        }
        if bonus_url > 0 {
            sufixo.push_str(" [URL tÃƒÂ©cnica]");
        }
        return Sugestao {
            categoria: Categoria::Util,
            confianca,
            motivo: format!(
                "{} palavras-chave de {}: {}{}{}",
                n,
                nome,
                exemplos,
                if hits.len() > 4 { "..." } else { "" },
                sufixo,
            ),
        };
    }

    if score_final >= 40 {
        let motivo = if motivos_ruins.is_empty() {
            "sem conteÃƒÂºdo tÃƒÂ©cnico identificÃƒÂ¡vel".to_string()
        } else {
            motivos_ruins.join("; ")
        };
        let confianca = (40i32 + score_final.min(55)).clamp(30, 95) as u8;
        return Sugestao {
            categoria: Categoria::Inutil,
            confianca,
            motivo,
        };
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Zona cinzenta Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    if n >= 3 {
        let exemplos = hits
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", ");
        let mut sufixo = String::new();
        if score_final < 0 {
            sufixo.push_str(&format!(" [score: {}]", score_final));
        }
        if bonus_tamanho > 0 {
            sufixo.push_str(&format!(" [{} bytes]", doc.content_length));
        }
        if bonus_url > 0 {
            sufixo.push_str(" [URL tÃƒÂ©cnica]");
        }
        Sugestao {
            categoria: Categoria::Util,
            confianca: 45,
            motivo: format!(
                "Sinal misto Ã¢â‚¬â€ palavras: {}; problemas: {}{}",
                exemplos,
                if motivos_ruins.is_empty() {
                    "nenhum".to_string()
                } else {
                    motivos_ruins.join(", ")
                },
                sufixo,
            ),
        }
    } else if n >= 1 {
        if score_final < 0 {
            let confianca = (60i32 + (-score_final / 5).min(25)).clamp(30, 88) as u8;
            let mut sufixo = String::new();
            if score_final < 0 {
                sufixo.push_str(&format!(" [score: {}]", score_final));
            }
            if bonus_tamanho > 0 {
                sufixo.push_str(&format!(" [{} bytes]", doc.content_length));
            }
            if bonus_url > 0 {
                sufixo.push_str(" [URL tÃƒÂ©cnica]");
            }
            Sugestao {
                categoria: Categoria::Util,
                confianca,
                motivo: format!(
                    "sinais positivos dominam (score {}); palavras: {}{}",
                    score_final,
                    hits.iter()
                        .map(|p| format!("\"{}\"", p))
                        .collect::<Vec<_>>()
                        .join(", "),
                    sufixo,
                ),
            }
        } else {
            Sugestao {
                categoria: Categoria::Inutil,
                confianca: 58,
                motivo: format!(
                    "conteÃƒÂºdo insuficiente para classificar com seguranÃƒÂ§a; {}",
                    if motivos_ruins.is_empty() {
                        "sem problemas estruturais identificados".to_string()
                    } else {
                        motivos_ruins.join(", ")
                    }
                ),
            }
        }
    } else {
        Sugestao {
            categoria: Categoria::Inutil,
            confianca: 55,
            motivo: format!(
                "Sem palavras-chave de {}{}",
                nome,
                if motivos_ruins.is_empty() {
                    String::new()
                } else {
                    format!("; {}", motivos_ruins.join(", "))
                }
            ),
        }
    }
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// TAXONOMIA E BROWSER
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

struct DocClassificado {
    pub id: String,
    pub source: String,
    pub domain: String,
    status: String,
    tags: Vec<String>,
    #[allow(dead_code)]
    pub content_length: i32,
}

fn db_salvar_tags(client: &mut Client, id: &str, tags: &[String]) {
    let arr: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
    client
        .execute(
            "UPDATE validation SET tags = $2 WHERE document_id::text = $1",
            &[&id, &arr],
        )
        .unwrap_or_else(|e| {
            eprintln!("[AVISO] tags: {}", e);
            0
        });
}

fn db_mover_documento(client: &mut Client, id: &str, novo_status: &str) {
    if novo_status == "approved" {
        client
            .execute(
                "UPDATE validation SET status = 'approved', decided_by = 'human', \
             decided_at = NOW(), rejection_reason = NULL \
             WHERE document_id::text = $1",
                &[&id],
            )
            .expect("Erro ao mover para aprovado");
    } else {
        client
            .execute(
                "UPDATE validation SET status = 'rejected', decided_by = 'human', \
             decided_at = NOW() WHERE document_id::text = $1",
                &[&id],
            )
            .expect("Erro ao mover para rejeitado");
    }
}

fn db_buscar_classificados(
    client: &mut Client,
    status_filter: &str,
    limit: i64,
    offset: i64,
) -> Vec<DocClassificado> {
    let rows = match status_filter {
        "approved" => client.query(
            "SELECT d.id::text, d.source, d.domain, v.status, \
             COALESCE(v.tags, ARRAY[]::text[]), d.content_length \
             FROM documents d JOIN validation v ON v.document_id = d.id \
             WHERE v.status = 'approved' ORDER BY v.decided_at DESC \
             LIMIT $1 OFFSET $2",
            &[&limit, &offset],
        ),
        "rejected" => client.query(
            "SELECT d.id::text, d.source, d.domain, v.status, \
             COALESCE(v.tags, ARRAY[]::text[]), d.content_length \
             FROM documents d JOIN validation v ON v.document_id = d.id \
             WHERE v.status = 'rejected' ORDER BY v.decided_at DESC \
             LIMIT $1 OFFSET $2",
            &[&limit, &offset],
        ),
        _ => client.query(
            "SELECT d.id::text, d.source, d.domain, v.status, \
             COALESCE(v.tags, ARRAY[]::text[]), d.content_length \
             FROM documents d JOIN validation v ON v.document_id = d.id \
             WHERE v.status IN ('approved','rejected') ORDER BY v.decided_at DESC \
             LIMIT $1 OFFSET $2",
            &[&limit, &offset],
        ),
    }
    .expect("Erro ao buscar classificados");

    rows.iter()
        .map(|r| DocClassificado {
            id: r.get(0),
            source: r.get(1),
            domain: r.get(2),
            status: r.get(3),
            tags: r.get::<_, Vec<String>>(4),
            content_length: r.get(5),
        })
        .collect()
}

fn db_contar_classificados(client: &mut Client, status_filter: &str) -> i64 {
    let row = match status_filter {
        "approved" => client.query_one(
            "SELECT COUNT(*) FROM validation WHERE status = 'approved'",
            &[],
        ),
        "rejected" => client.query_one(
            "SELECT COUNT(*) FROM validation WHERE status = 'rejected'",
            &[],
        ),
        _ => client.query_one(
            "SELECT COUNT(*) FROM validation WHERE status IN ('approved','rejected')",
            &[],
        ),
    }
    .expect("Erro ao contar");
    row.get::<_, i64>(0)
}

fn sugerir_tags(doc: &Documento, util: bool) -> Vec<String> {
    gerar_tags_por_url(&doc.source, &doc.domain, util)
}

fn prompt_tags(doc: &Documento, util: bool, stdin: &io::Stdin) -> Vec<String> {
    let sugeridas = sugerir_tags(doc, util);
    println!("  Tags sugeridas : {}", sugeridas.join(", "));
    print!("  [Enter] aceitar | editar (sep. por virgula): ");
    io::stdout().flush().expect("flush");
    let mut buf = String::new();
    stdin.lock().read_line(&mut buf).expect("read");
    let linha = buf.trim().to_string();
    if linha.is_empty() {
        sugeridas
    } else {
        linha
            .split(',')
            .map(|t| t.trim().to_lowercase().replace(' ', "_"))
            .filter(|t| !t.is_empty())
            .collect()
    }
}

fn exibir_linha_classificado(doc: &DocClassificado, sel: bool) {
    let status_label = if doc.status == "approved" {
        "UTIL  "
    } else {
        "INUTEL"
    };
    let tags_str = if doc.tags.is_empty() {
        "(sem tags)".to_string()
    } else {
        doc.tags.join(", ")
    };
    let prefix = if sel { ">" } else { " " };
    let src: String = if doc.source.len() > 68 {
        format!("...{}", &doc.source[doc.source.len().saturating_sub(65)..])
    } else {
        doc.source.clone()
    };
    let tags_display: String = if largura_visual(&tags_str) > 55 {
        format!(
            "{}...",
            &tags_str[..tags_str
                .char_indices()
                .nth(52)
                .map(|(i, _)| i)
                .unwrap_or(tags_str.len())]
        )
    } else {
        tags_str
    };
    println!(
        "{} [{}] {} | {}",
        prefix, status_label, doc.domain, tags_display
    );
    println!("        {}", src);
    println!();
}

fn autoclassificar_todos(client: &mut Client) -> usize {
    let rows = client
        .query(
            "SELECT d.id::text, d.source, d.domain, v.status \
         FROM documents d JOIN validation v ON v.document_id = d.id \
         WHERE v.status IN ('approved','rejected') \
           AND (v.tags IS NULL OR v.tags = '{}' OR array_length(v.tags, 1) IS NULL) \
         ORDER BY v.decided_at",
            &[],
        )
        .expect("Erro ao buscar docs sem tags");

    let mut count = 0usize;
    for row in &rows {
        let id: String = row.get(0);
        let source: String = row.get(1);
        let domain: String = row.get(2);
        let status: String = row.get(3);
        let util = status == "approved";
        let tags = gerar_tags_por_url(&source, &domain, util);
        let arr: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
        client
            .execute(
                "UPDATE validation SET tags = $2 WHERE document_id::text = $1",
                &[&id, &arr],
            )
            .unwrap_or_else(|e| {
                eprintln!("[AVISO] {}", e);
                0
            });
        count += 1;
    }
    count
}

fn gerar_tags_por_url(source: &str, domain: &str, util: bool) -> Vec<String> {
    let src = source.to_lowercase();
    let mut t: Vec<String> = Vec::new();

    t.push(if util {
        "util".to_string()
    } else {
        "inutil".to_string()
    });
    t.push(domain.to_string());

    if src.contains("kernel.org") {
        t.push("kernel".to_string());
        if src.contains("driver-api") || src.contains("/driver") {
            t.push("drivers".to_string());
        }
        if src.contains("/gpu") || src.contains("drm-") || src.contains("drm/") {
            t.push("gpu".to_string());
        }
        if src.contains("amdgpu") || src.contains("/amd/") {
            t.push("amd".to_string());
        }
        if src.contains("i915") || src.contains("/intel/") {
            t.push("intel".to_string());
        }
        if src.contains("nouveau") {
            t.push("nvidia".to_string());
        }
        if src.contains("fpga") {
            t.push("fpga".to_string());
        }
        if src.contains("/cxl") {
            t.push("cxl".to_string());
        }
        if src.contains("auxiliary") {
            t.push("auxiliary_bus".to_string());
        }
        if src.contains("/iio/") {
            t.push("sensors".to_string());
        }
        if src.contains("/spi") {
            t.push("spi".to_string());
        }
        if src.contains("/i2c") {
            t.push("i2c".to_string());
        }
        if src.contains("/usb") {
            t.push("usb".to_string());
        }
        if src.contains("/pci") {
            t.push("pci".to_string());
        }
        if src.contains("networking") || src.contains("net/") {
            t.push("networking".to_string());
        }
        if src.contains("memory-management") || src.contains("/mm/") {
            t.push("memoria".to_string());
        }
        if src.contains("filesystems") || src.contains("/fs/") {
            t.push("filesystem".to_string());
        }
        if src.contains("locking") {
            t.push("locking".to_string());
        }
        if src.contains("sound") || src.contains("alsa") {
            t.push("audio".to_string());
        }
        if src.contains("core-api") {
            t.push("core_api".to_string());
        }
        if src.contains("dev-tools") {
            t.push("dev_tools".to_string());
        }
        if src.contains("dma-buf") || src.contains("dma_buf") {
            t.push("dma_buf".to_string());
        }
        if src.contains("libata") {
            t.push("storage".to_string());
        }
        if src.contains("scsi") {
            t.push("scsi".to_string());
        }
        if src.contains("regulator") {
            t.push("power".to_string());
        }
        if src.contains("thermal") {
            t.push("thermal".to_string());
        }
        if src.contains("gpio") {
            t.push("gpio".to_string());
        }
        if src.contains("mmc") || src.contains("sdio") {
            t.push("mmc".to_string());
        }
        if src.contains("virtio") {
            t.push("virtio".to_string());
        }
        if src.contains("iommu") {
            t.push("iommu".to_string());
        }
        if !util && (src.contains("genindex") || src.contains("index.html") || src.contains("/toc")) {
            t.push("indice".to_string());
        }
    } else if src.contains("docker.com") || src.contains("docs.docker") {
        t.push("docker".to_string());
        if src.contains("network") {
            t.push("networking".to_string());
        }
        if src.contains("volume") || src.contains("storage") {
            t.push("storage".to_string());
        }
        if src.contains("compose") {
            t.push("compose".to_string());
        }
        if src.contains("security") {
            t.push("security".to_string());
        }
        if src.contains("build") {
            t.push("build".to_string());
        }
        if src.contains("swarm") {
            t.push("swarm".to_string());
        }
    } else if src.contains("postgresql.org") {
        t.push("postgresql".to_string());
        if src.contains("query") || src.contains("sql") {
            t.push("sql".to_string());
        }
        if src.contains("backup") {
            t.push("backup".to_string());
        }
        if src.contains("replication") {
            t.push("replicacao".to_string());
        }
        if src.contains("performance") {
            t.push("performance".to_string());
        }
        if src.contains("index") {
            t.push("indexacao".to_string());
        }
        if src.contains("trigger") {
            t.push("triggers".to_string());
        }
        if src.contains("function") {
            t.push("functions".to_string());
        }
    } else if src.contains("systemd.io") || src.contains("/systemd/") {
        t.push("systemd".to_string());
        if src.contains("network") {
            t.push("networking".to_string());
        }
        if src.contains("service") {
            t.push("services".to_string());
        }
        if src.contains("journal") {
            t.push("logging".to_string());
        }
        if src.contains("cgroup") {
            t.push("cgroups".to_string());
        }
        if src.contains("socket") {
            t.push("sockets".to_string());
        }
        if src.contains("timer") {
            t.push("timers".to_string());
        }
        if src.contains("mount") {
            t.push("mount".to_string());
        }
    } else if src.contains("doc.rust-lang.org") {
        if src.contains("nomicon") {
            t.push("nomicon".to_string());
        } else if src.contains("reference") {
            t.push("reference".to_string());
        } else if src.contains("/book/") {
            t.push("book".to_string());
        } else if src.contains("std/") {
            t.push("stdlib".to_string());
        } else if src.contains("cargo") {
            t.push("cargo".to_string());
        } else {
            t.push("rust_docs".to_string());
        }
    } else if src.contains("docs.rs/") {
        t.push("api_reference".to_string());
        let parts: Vec<&str> = src.split('/').collect();
        if let Some(i) = parts.iter().position(|&p| p == "docs.rs")
            && let Some(cn) = parts.get(i + 1)
        {
            let cn = cn.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
            if !cn.is_empty() {
                t.push(cn.to_string());
            }
        }
    } else if src.contains("owasp.org") {
        t.push("owasp".to_string());
        if src.contains("top10") || src.contains("top-10") {
            t.push("top10".to_string());
        }
        if src.contains("cheat") {
            t.push("cheatsheet".to_string());
        }
    } else if src.contains("nist.gov") {
        t.push("nist".to_string());
        if src.contains("nvd") || src.contains("cve") {
            t.push("cve".to_string());
        }
        if src.contains("800-") {
            t.push("sp800".to_string());
        }
    } else if src.contains("arxiv.org") {
        t.push("paper".to_string());
        if src.contains("qlora") || src.contains("lora") {
            t.push("lora".to_string());
        }
        if src.contains("attention") || src.contains("transformer") {
            t.push("transformers".to_string());
        }
    } else if src.contains("huggingface.co") {
        t.push("huggingface".to_string());
        if src.contains("peft") {
            t.push("peft".to_string());
        }
        if src.contains("transformers") {
            t.push("transformers".to_string());
        }
        if src.contains("trl") {
            t.push("trl".to_string());
        }
        if src.contains("tokenizers") {
            t.push("tokenizers".to_string());
        }
        if src.contains("datasets") {
            t.push("datasets".to_string());
        }
        if src.contains("accelerate") {
            t.push("accelerate".to_string());
        }
    } else if src.contains("llama") || src.contains("gguf") {
        t.push("llama_cpp".to_string());
    } else if src.contains("ietf.org") || src.contains("/rfc") {
        t.push("rfc".to_string());
        if src.contains("8446") {
            t.push("tls13".to_string());
        }
        if src.contains("9293") {
            t.push("tcp".to_string());
        }
    }

    t.dedup();
    t
}

fn exibir_documento_revalidar(doc: &Documento, idx: usize, total: usize, n_linhas: usize) {
    limpar_tela();
    let conteudo_header = format!(
        "  Lote: {}/{}   DomÃƒÂ­nio: {}   Fonte: {:.40}",
        idx + 1,
        total,
        doc.domain,
        doc.source
    );
    let padding = 62usize.saturating_sub(largura_visual(&conteudo_header));
    println!("Ã¢â€¢â€Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢â€”");
    println!("Ã¢â€¢â€˜  NEXUS Ã¢â‚¬â€ REVALIDADOR IA                                     Ã¢â€¢â€˜");
    println!("Ã¢â€¢Â Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â£");
    println!("Ã¢â€¢â€˜{}{}Ã¢â€¢â€˜", conteudo_header, " ".repeat(padding));
    println!("Ã¢â€¢Å¡Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â");
    println!();
    println!("  DomÃƒÂ­nio  : {}", doc.domain);
    println!("  Tamanho  : {} bytes", doc.content_length);
    println!("  Linhas   : {}", n_linhas);
    println!("  Fonte    : {}", doc.source);
    println!();
    println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
    println!("  PRÃƒâ€°VIA DO CONTEÃƒÅ¡DO (30 linhas):");
    println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
    println!();
    for linha in doc.preview.lines().take(30) {
        println!("  {}", linha);
    }
    println!();
    println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
}

fn exibir_resultado_revalidar(s: &Sugestao, heur: &Sugestao) {
    let barras_ia = (s.confianca / 10) as usize;
    let barra_ia = format!("{}{}", "Ã¢â€“Ë†".repeat(barras_ia), "Ã¢â€“â€˜".repeat(10 - barras_ia));
    let barras_heur = (heur.confianca / 10) as usize;
    let barra_heur = format!(
        "{}{}",
        "Ã¢â€“Ë†".repeat(barras_heur),
        "Ã¢â€“â€˜".repeat(10 - barras_heur)
    );

    let label_ia = if s.categoria == Categoria::Util {
        "APROVADO "
    } else {
        "REJEITADO"
    };
    let label_heur = if heur.categoria == Categoria::Util {
        "UTIL     "
    } else {
        "INUTIL   "
    };
    let conf_ia_str = format!("{:3}%", s.confianca);
    let conf_heur_str = format!("{:3}%", heur.confianca);

    let linha_ia = format!("{} {} [{}]", label_ia, conf_ia_str, barra_ia);
    let linha_heur = format!("{} {} [{}]", label_heur, conf_heur_str, barra_heur);
    let pad_ia = 62usize.saturating_sub(largura_visual(&linha_ia));
    let pad_heur = 62usize.saturating_sub(largura_visual(&linha_heur));

    println!("  Ã¢â€Å’Ã¢â€â‚¬ IA Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â");
    println!("  Ã¢â€â€š  {}{}Ã¢â€â€š", linha_ia, " ".repeat(pad_ia));
    println!("  Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ");
    println!("  Ã¢â€Å’Ã¢â€â‚¬ HeurÃƒÂ­stica Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â");
    println!("  Ã¢â€â€š  {}{}Ã¢â€â€š", linha_heur, " ".repeat(pad_heur));
    println!("  Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ");
    println!();
    // Caixa de resumo
    let linhas_motivo = quebrar_motivo(&s.motivo, 56);
    println!("  Ã¢â€Å’Ã¢â€â‚¬ Resumo do Documento Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â");
    println!("  Ã¢â€â€š                                                              Ã¢â€â€š");
    for l in &linhas_motivo {
        let p = 56usize.saturating_sub(largura_visual(l));
        println!("  Ã¢â€â€š  {}{}  Ã¢â€â€š", l, " ".repeat(p));
    }
    println!("  Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ");
}

fn browser_tui(client: &mut Client, stdin: &io::Stdin) {
    const PAGE: i64 = 8;
    let mut offset: i64 = 0;
    let mut filtro: String = "all".to_string();
    let mut sel: usize = 0;

    loop {
        let total = db_contar_classificados(client, &filtro);
        let docs = db_buscar_classificados(client, &filtro, PAGE, offset);

        if docs.is_empty() && offset > 0 {
            offset = 0;
            continue;
        }

        limpar_tela();
        let total_pags = if total == 0 {
            1
        } else {
            (total - 1) / PAGE + 1
        };
        let pag_atual = offset / PAGE + 1;
        let fl = match filtro.as_str() {
            "approved" => "uteis",
            "rejected" => "inuteis",
            _ => "todos",
        };
        let header = format!(
            "  Filtro: {} | {} docs | pag {}/{}",
            fl, total, pag_atual, total_pags
        );
        let pad = 62usize.saturating_sub(largura_visual(&header));

        println!("Ã¢â€¢â€Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢â€”");
        println!("Ã¢â€¢â€˜  NEXUS Ã¢â‚¬â€ BROWSER DE DOCUMENTOS CLASSIFICADOS                Ã¢â€¢â€˜");
        println!("Ã¢â€¢Â Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â£");
        println!("Ã¢â€¢â€˜{}{}Ã¢â€¢â€˜", header, " ".repeat(pad));
        println!("Ã¢â€¢Å¡Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â");
        println!();

        if docs.is_empty() {
            println!("  Nenhum documento encontrado.");
        } else {
            for (i, doc) in docs.iter().enumerate() {
                exibir_linha_classificado(doc, i == sel);
            }
        }

        println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
        println!("  [j/k] navegar  [n] prox pag  [p] pag ant");
        println!("  [u] uteis  [i] inuteis  [a] todos");
        println!("  [m] mover util/inutil  [v] ver doc  [t] editar tags");
        println!("  [c] autoclassificar sem tags  [q] sair");
        println!("Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬");
        print!("  Comando: ");
        io::stdout().flush().expect("flush");

        let cmd = ler_linha(stdin);
        let nd = docs.len();

        match cmd.as_str() {
            "q" => break,
            "u" => {
                filtro = "approved".to_string();
                offset = 0;
                sel = 0;
            }
            "i" => {
                filtro = "rejected".to_string();
                offset = 0;
                sel = 0;
            }
            "a" => {
                filtro = "all".to_string();
                offset = 0;
                sel = 0;
            }
            "n" => {
                if offset + PAGE < total {
                    offset += PAGE;
                    sel = 0;
                }
            }
            "p" => {
                if offset >= PAGE {
                    offset -= PAGE;
                    sel = 0;
                }
            }
            "j" => {
                if nd > 0 && sel < nd - 1 {
                    sel += 1;
                }
            }
            "k" => {
                sel = sel.saturating_sub(1);
            }
            "m" => {
                if nd > 0 && sel < nd {
                    let doc = &docs[sel];
                    let novo = if doc.status == "approved" {
                        "rejected"
                    } else {
                        "approved"
                    };
                    db_mover_documento(client, &doc.id, novo);
                    let label = if novo == "approved" { "UTIL" } else { "INUTIL" };
                    println!("  Movido para {}.", label);
                    std::thread::sleep(std::time::Duration::from_millis(600));
                }
            }
            "v" => {
                if nd > 0 && sel < nd {
                    let conteudo = buscar_conteudo_completo(client, &docs[sel].id);
                    exibir_conteudo_completo(&conteudo, &docs[sel].source, stdin);
                }
            }
            "t" => {
                if nd > 0 && sel < nd {
                    let doc = &docs[sel];
                    println!("  Tags atuais : {}", doc.tags.join(", "));
                    println!("  [q] cancelar | ou digite novas tags (sep. por virgula):");
                    print!("  > ");
                    io::stdout().flush().expect("flush");
                    let mut buf = String::new();
                    stdin.lock().read_line(&mut buf).expect("read");
                    let entrada = buf.trim().to_string();
                    if entrada == "q" || entrada == "Q" {
                        println!("  Cancelado.");
                        std::thread::sleep(std::time::Duration::from_millis(400));
                    } else if !entrada.is_empty() {
                        let novas: Vec<String> = entrada
                            .split(',')
                            .map(|t| t.trim().to_lowercase().replace(' ', "_"))
                            .filter(|t| !t.is_empty())
                            .collect();
                        if !novas.is_empty() {
                            db_salvar_tags(client, &doc.id, &novas);
                            println!("  Tags atualizadas: {}", novas.join(", "));
                            std::thread::sleep(std::time::Duration::from_millis(600));
                        }
                    }
                }
            }
            "c" => {
                let sem_tags = client.query_one(
                    "SELECT COUNT(*) FROM validation                      WHERE status IN ('approved','rejected')                        AND (tags IS NULL OR tags = '{}' OR array_length(tags, 1) IS NULL)",
                    &[],
                ).map(|r| r.get::<_, i64>(0)).unwrap_or(0);
                if sem_tags == 0 {
                    println!("  Todos os documentos ja possuem tags.");
                } else {
                    println!(
                        "  {} documentos sem tags. Autoclassificar? [s/N]: ",
                        sem_tags
                    );
                    io::stdout().flush().expect("flush");
                    let mut buf = String::new();
                    stdin.lock().read_line(&mut buf).expect("read");
                    if buf.trim().to_lowercase() == "s" {
                        println!("  Classificando...");
                        let n = autoclassificar_todos(client);
                        println!("  {} documentos classificados.", n);
                    } else {
                        println!("  Cancelado.");
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(800));
            }
            _ => {}
        }
    }
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// MAIN
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

fn merge_tags_unicas(existing: Vec<String>, extras: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for t in existing.into_iter().chain(extras.iter().cloned()) {
        let tag = t.trim().to_lowercase();
        if !tag.is_empty() && seen.insert(tag.clone()) {
            out.push(tag);
        }
    }
    out
}

fn slug_motivo_recheck(motivo: &str) -> String {
    let low = motivo.to_lowercase();
    if low.contains("eliminat") || low.contains("indice") || low.contains("toc") {
        "indice_navegacao".to_string()
    } else if low.contains("idioma") {
        "idioma_header".to_string()
    } else if low.contains("insuficiente") || low.contains("sem palavras-chave") {
        "insuficiente".to_string()
    } else if low.contains("pontua") || low.contains("linhas") {
        "estrutura_ruim".to_string()
    } else {
        "outro".to_string()
    }
}

fn recheck_codex_approved(client: &mut Client) {
    let rows = client
        .query(
            "SELECT d.id::text, d.source, d.domain, d.doc_type, d.content_length,
                    LEFT(convert_from(convert_to(d.content,'UTF8'),'UTF8'), 4000),
                    RIGHT(convert_from(convert_to(d.content,'UTF8'),'UTF8'), 1000),
                    COALESCE(v.tags, ARRAY[]::text[])
             FROM documents d
             JOIN validation v ON v.document_id = d.id
             WHERE v.status = 'approved' AND v.decided_by = 'codex'
             ORDER BY d.domain, d.collected_at",
            &[],
        )
        .expect("Erro ao buscar aprovados do codex");

    if rows.is_empty() {
        println!("Nenhum documento com status=approved e decided_by=codex.");
        return;
    }

    let mut total = 0usize;
    let mut mantidos = 0usize;
    let mut rebaixados = 0usize;
    let mut por_dominio_keep: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut por_dominio_pending: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut motivos_pending: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    for row in &rows {
        total += 1;
        let id: String = row.get(0);
        let source: String = row.get(1);
        let domain: String = row.get(2);
        let doc_type: String = row.get(3);
        let content_length: i32 = row.get(4);
        let raw: String = row.get(5);
        let tail: String = row.get(6);
        let tags: Vec<String> = row.get(7);

        let head: String = raw.chars().take(1000).collect();
        let preview = filtrar_preview(&raw);
        let doc = Documento {
            id: id.clone(),
            source: source.clone(),
            domain: domain.clone(),
            doc_type,
            content_length,
            preview,
            content: tail,
            head,
        };

        let heur = sugerir_heuristica_interna(&doc);
        let heur_tag = format!("heur_recheck:{}", heur.confianca);
        let manter_aprovado = heur.categoria == Categoria::Util && heur.confianca >= 60;

        if manter_aprovado {
            let merged = merge_tags_unicas(
                tags,
                &[
                    "codex_validated".to_string(),
                    "codex_recheck_ok".to_string(),
                    heur_tag,
                ],
            );
            db_salvar_tags(client, &id, &merged);
            mantidos += 1;
            *por_dominio_keep.entry(domain).or_insert(0) += 1;
        } else {
            let motivo_slug = slug_motivo_recheck(&heur.motivo);
            let merged = merge_tags_unicas(
                tags,
                &[
                    "codex_validated".to_string(),
                    "codex_recheck_needed".to_string(),
                    "codex_recheck_heuristica".to_string(),
                    "codex_pending".to_string(),
                    motivo_slug.clone(),
                    heur_tag,
                ],
            );
            let arr: Vec<&str> = merged.iter().map(|s| s.as_str()).collect();
            client
                .execute(
                    "UPDATE validation
                     SET status = 'pending',
                         decided_by = 'pending',
                         decided_at = NULL,
                         rejection_reason = NULL,
                         tags = $2
                     WHERE document_id::text = $1",
                    &[&id, &arr],
                )
                .expect("Erro ao rebaixar aprovado do codex para pending");
            rebaixados += 1;
            *por_dominio_pending.entry(domain).or_insert(0) += 1;
            *motivos_pending.entry(motivo_slug).or_insert(0) += 1;
        }

        if total.is_multiple_of(100) {
            println!(
                "[PROGRESSO] processados={} mantidos={} rebaixados={}",
                total, mantidos, rebaixados
            );
        }
    }

    println!();
    println!("===== RECHECK CODEX (HEURISTICA) =====");
    println!("Total analisado : {}", total);
    println!("Mantidos aprov. : {}", mantidos);
    println!("Voltaram pending: {}", rebaixados);
    println!();
    println!("Por dominio:");
    for (d, keep) in &por_dominio_keep {
        let pend = por_dominio_pending.get(d).copied().unwrap_or(0);
        println!("  {:<10} keep={} pending={}", d, keep, pend);
    }
    for (d, pend) in &por_dominio_pending {
        if !por_dominio_keep.contains_key(d) {
            println!("  {:<10} keep=0 pending={}", d, pend);
        }
    }
    println!();
    println!("Motivos de rebaixamento (top):");
    let mut motivos: Vec<(&String, &usize)> = motivos_pending.iter().collect();
    motivos.sort_by(|a, b| b.1.cmp(a.1));
    for (m, n) in motivos.into_iter().take(10) {
        println!("  {:<22} {}", m, n);
    }
}

fn main() {
    let senha = std::env::var("KB_INGEST_PASSWORD").unwrap_or_else(|_| {
        eprintln!("[ERRO] KB_INGEST_PASSWORD nÃƒÂ£o definida.");
        std::process::exit(1);
    });

    let pg_host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "172.23.160.1".to_string());
    let pg_port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5433".to_string());
    let pg_db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "knowledge_base".to_string());
    let pg_user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "kb_ingest".to_string());

    let conn_str = format!(
        "host={} port={} dbname={} user={} password={}",
        pg_host, pg_port, pg_db, pg_user, senha
    );

    let mut client = Client::connect(&conn_str, NoTls).unwrap_or_else(|e| {
        eprintln!("[ERRO] Falha ao conectar ao banco de dados: {}", e);
        std::process::exit(1);
    });

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--recheck-codex") {
        recheck_codex_approved(&mut client);
        return;
    }

    let stdin = io::stdin();
    let mut config = carregar_config();
    let mut pulados_sessao = carregar_sessao();
    let sessao_anterior_pulados = pulados_sessao.len();

    // Ã¢â€â‚¬Ã¢â€â‚¬ Carrega estado completo da sessÃƒÂ£o anterior Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    let estado_anterior = carregar_estado_sessao();

    let mut aprovados: u32;
    let mut rejeitados: u32;
    let mut pulados_conta: u32;
    let started_at: String;

    if let Some(ref estado) = estado_anterior {
        limpar_tela();
        println!();
        println!("  Ã¢â€Å’Ã¢â€â‚¬ SESSÃƒÆ’O ANTERIOR ENCONTRADA Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â");
        println!("  Ã¢â€â€š  Iniciada em: {}  Ã¢â€â€š", &estado.started_at[..19]);
        println!("  Ã¢â€â€š  Aprovados  : {}    Ã¢â€â€š", estado.aprovados);
        println!("  Ã¢â€â€š  Rejeitados : {}    Ã¢â€â€š", estado.rejeitados);
        println!("  Ã¢â€â€š  Pulados    : {}    Ã¢â€â€š", estado.pulados);
        if let Some(ref uid) = estado.ultimo_documento_id {
            println!("  Ã¢â€â€š  ÃƒÅ¡ltimo ID  : {}  Ã¢â€â€š", uid);
        }
        println!("  Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ");
        println!();
        print!("  Continuar sessÃƒÂ£o? [Enter] ou nova sessÃƒÂ£o [n]: ");
        io::stdout()
            .flush()
            .expect("Erro ao exibir prompt de sessÃƒÂ£o");
        let resp = ler_linha(&stdin);
        if resp == "n" {
            limpar_sessao();
            pulados_sessao.clear();
            aprovados = 0;
            rejeitados = 0;
            pulados_conta = 0;
            started_at = Local::now().to_rfc3339();
        } else {
            aprovados = estado.aprovados;
            rejeitados = estado.rejeitados;
            pulados_conta = estado.pulados;
            started_at = estado.started_at.clone();
        }
    } else if sessao_anterior_pulados > 0 {
        // Existe apenas o arquivo de pulados (sessÃƒÂ£o antiga sem JSON de estado)
        limpar_tela();
        println!();
        println!(
            "  SessÃƒÂ£o anterior: {} documentos pulados lembrados.",
            sessao_anterior_pulados
        );
        print!("  Continuar? [Enter] ou limpar [l]: ");
        io::stdout()
            .flush()
            .expect("Erro ao exibir prompt de sessÃƒÂ£o legada");
        let resp = ler_linha(&stdin);
        if resp == "l" {
            limpar_sessao();
            pulados_sessao.clear();
        }
        aprovados = 0;
        rejeitados = 0;
        pulados_conta = 0;
        started_at = Local::now().to_rfc3339();
    } else {
        aprovados = 0;
        rejeitados = 0;
        pulados_conta = 0;
        started_at = Local::now().to_rfc3339();
    }

    let mut historico: Vec<HistoricoItem> = Vec::new();
    let mut modo_sugestao = ModoSugestao::Desligado;
    let parar_auto = Arc::new(AtomicBool::new(false));
    let inicio_sessao = std::time::Instant::now();

    macro_rules! persistir {
        ($id:expr) => {
            salvar_estado_sessao(&EstadoSessao {
                started_at: started_at.clone(),
                aprovados,
                rejeitados,
                pulados: pulados_conta,
                ultimo_documento_id: Some($id.to_string()),
            });
        };
    }

    'principal: loop {
        let docs = buscar_lote(&mut client, &pulados_sessao, config.tamanho_lote);

        if docs.is_empty() {
            limpar_tela();
            println!("Ã¢â€¢â€Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢â€”");
            println!("Ã¢â€¢â€˜     VALIDAÃƒâ€¡ÃƒÆ’O CONCLUÃƒÂDA              Ã¢â€¢â€˜");
            println!("Ã¢â€¢Å¡Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â");
            println!();
            println!("  Aprovados : {}", aprovados);
            println!("  Rejeitados: {}", rejeitados);
            println!("  Pulados   : {}", pulados_conta);
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
                &modo_sugestao,
            );

            match &modo_sugestao {
                ModoSugestao::Heuristica => {
                    let s = sugerir_heuristica(doc);
                    exibir_sugestao(&s);
                }
                ModoSugestao::IA => {
                    // Roda Ollama em thread separada para nÃƒÂ£o bloquear stdin
                    let dom_clone = doc.domain.clone();
                    let cont_clone = doc.preview.clone(); // preview jÃƒÂ¡ filtrado: sem nav, sem duplicatas
                    let timeout_clone = config.timeout_ollama;
                    let handle = std::thread::spawn(move || {
                        sugerir_ia(&dom_clone, &cont_clone, timeout_clone)
                    });

                    // Thread separada lÃƒÂª stdin para capturar [x]
                    let parar_stdin = Arc::clone(&parar_auto);
                    let stdin_handle = std::thread::spawn(move || {
                        let stdin = io::stdin();
                        let mut buf = String::new();
                        stdin.lock().read_line(&mut buf).ok();
                        if buf.trim() == "x" {
                            parar_stdin.store(true, Ordering::Relaxed);
                        }
                    });

                    // Polling do resultado do Ollama
                    print!("  [IA] Processando (x+Enter para parar)");
                    io::stdout().flush().ok();
                    let mut dots = 0usize;
                    loop {
                        if handle.is_finished() {
                            break;
                        }
                        if parar_auto.load(Ordering::Relaxed) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        print!(".");
                        io::stdout().flush().ok();
                        dots += 1;
                        if dots > (config.timeout_ollama * 1000 / 300 + 10) as usize {
                            break;
                        }
                    }
                    println!();

                    // Se parado pelo usuÃƒÂ¡rio, descarta resultado
                    let resultado_ia = if parar_auto.load(Ordering::Relaxed) {
                        drop(stdin_handle);
                        None
                    } else {
                        drop(stdin_handle);
                        handle.join().ok().flatten()
                    };

                    if parar_auto.load(Ordering::Relaxed) {
                        parar_auto.store(false, Ordering::Relaxed);
                        modo_sugestao = ModoSugestao::Desligado;
                        // Redesenha o documento para o usuÃƒÂ¡rio nÃƒÂ£o ficar sem contexto
                        let pendentes_re = contar_pendentes(&mut client, &pulados_sessao);
                        exibir_documento(
                            doc,
                            idx,
                            total_lote,
                            aprovados,
                            rejeitados,
                            pulados_conta,
                            pendentes_re,
                            config.linhas_preview,
                            &modo_sugestao,
                        );
                        println!("  [IA] Auto-IA interrompida. Decida manualmente.");
                    } else if let Some(s) = resultado_ia {
                        let heur = sugerir_heuristica(doc);
                        exibir_sugestao(&s);

                        // HeurÃƒÂ­stica < 60: pede decisÃƒÂ£o manual com timeout 30s
                        if heur.confianca < config.threshold_heuristica {
                            println!(
                                "  [IA] HeurÃƒÂ­stica baixa ({}%) Ã¢â‚¬â€ sua decisÃƒÂ£o em 30s [a/r/u] ou pula:",
                                heur.confianca
                            );
                            io::stdout().flush().ok();

                            let (tx, rx) = std::sync::mpsc::channel();
                            std::thread::spawn(move || {
                                let stdin = io::stdin();
                                let mut buf = String::new();
                                stdin.lock().read_line(&mut buf).ok();
                                let _ = tx.send(buf.trim().to_string());
                            });

                            let decisao_manual = rx
                                .recv_timeout(std::time::Duration::from_secs(30))
                                .unwrap_or_else(|_| "p".to_string());

                            match decisao_manual.as_str() {
                                "a" => {
                                    db_aprovar(&mut client, &doc.id);
                                    let mut tags_auto = sugerir_tags(doc, true);
                                    tags_auto.push(format!("heur:{}", heur.confianca));
                                    db_salvar_tags(&mut client, &doc.id, &tags_auto);
                                    historico.push(HistoricoItem {
                                        id: doc.id.clone(),
                                        acao: Acao::Aprovado,
                                    });
                                    aprovados += 1;
                                    persistir!(&doc.id);
                                    idx += 1;
                                    println!("  Aprovado manualmente.");
                                }
                                "r" | "u" => {
                                    let motivo = if decisao_manual == "u" {
                                        "conteÃƒÂºdo inÃƒÂºtil".to_string()
                                    } else {
                                        format!("[ia-suspeito] {}", s.motivo)
                                    };
                                    db_rejeitar(&mut client, &doc.id, &motivo);
                                    let mut tags_auto = sugerir_tags(doc, false);
                                    tags_auto.push(format!("heur:{}", heur.confianca));
                                    db_salvar_tags(&mut client, &doc.id, &tags_auto);
                                    historico.push(HistoricoItem {
                                        id: doc.id.clone(),
                                        acao: Acao::Rejeitado,
                                    });
                                    rejeitados += 1;
                                    persistir!(&doc.id);
                                    idx += 1;
                                    println!("  Rejeitado manualmente.");
                                }
                                _ => {
                                    println!("  [IA] Sem resposta Ã¢â‚¬â€ pulando.");
                                    idx += 1;
                                }
                            }
                            std::thread::sleep(std::time::Duration::from_millis(400));
                            continue;
                        }

                        if s.confianca >= config.threshold_ia {
                            let motivo_ia = format!("[ia] {}", s.motivo);
                            match s.categoria {
                                Categoria::Util => {
                                    db_aprovar_ia(&mut client, &doc.id);
                                    let mut tags_auto = sugerir_tags(doc, true);
                                    tags_auto.push(format!("heur:{}", heur.confianca));
                                    db_salvar_tags(&mut client, &doc.id, &tags_auto);
                                    historico.push(HistoricoItem {
                                        id: doc.id.clone(),
                                        acao: Acao::Aprovado,
                                    });
                                    aprovados += 1;
                                    persistir!(&doc.id);
                                    idx += 1;
                                    println!("  [IA] Aprovado automaticamente ({}%)", s.confianca);
                                    std::thread::sleep(std::time::Duration::from_millis(400));
                                    continue;
                                }
                                Categoria::Inutil => {
                                    db_rejeitar_ia(&mut client, &doc.id, &motivo_ia);
                                    let mut tags_auto = sugerir_tags(doc, false);
                                    tags_auto.push(format!("heur:{}", heur.confianca));
                                    db_salvar_tags(&mut client, &doc.id, &tags_auto);
                                    historico.push(HistoricoItem {
                                        id: doc.id.clone(),
                                        acao: Acao::Rejeitado,
                                    });
                                    rejeitados += 1;
                                    persistir!(&doc.id);
                                    idx += 1;
                                    println!("  [IA] Rejeitado automaticamente ({}%)", s.confianca);
                                    std::thread::sleep(std::time::Duration::from_millis(400));
                                    continue;
                                }
                            }
                        } else {
                            println!(
                                "  [IA] ConfianÃƒÂ§a baixa ({}%) Ã¢â‚¬â€ decisÃƒÂ£o manual necessÃƒÂ¡ria.",
                                s.confianca
                            );
                        }
                    } else {
                        println!("  [IA] Sem resposta da IA Ã¢â‚¬â€ pulando documento.");
                        std::thread::sleep(std::time::Duration::from_millis(600));
                        idx += 1;
                        continue;
                    }
                }
                ModoSugestao::Desligado => {}
            }
            print!("  DecisÃƒÂ£o: ");
            io::stdout()
                .flush()
                .expect("Erro ao exibir prompt de decisÃƒÂ£o");

            let decisao = ler_linha(&stdin);

            match decisao.as_str() {
                "a" => {
                    db_aprovar(&mut client, &doc.id);
                    let tags_a = prompt_tags(doc, true, &stdin);
                    db_salvar_tags(&mut client, &doc.id, &tags_a);
                    historico.push(HistoricoItem {
                        id: doc.id.clone(),
                        acao: Acao::Aprovado,
                    });
                    aprovados += 1;
                    persistir!(&doc.id);
                    idx += 1;
                }
                "r" => {
                    print!("  Motivo: ");
                    io::stdout()
                        .flush()
                        .expect("Erro ao exibir prompt de motivo");
                    let mut buf = String::new();
                    stdin
                        .lock()
                        .read_line(&mut buf)
                        .expect("Erro ao ler motivo de rejeiÃƒÂ§ÃƒÂ£o");
                    let motivo = buf.trim().to_string();
                    let motivo = if motivo.is_empty() {
                        "sem motivo".to_string()
                    } else {
                        motivo
                    };
                    db_rejeitar(&mut client, &doc.id, &motivo);
                    let tags_r = prompt_tags(doc, false, &stdin);
                    db_salvar_tags(&mut client, &doc.id, &tags_r);
                    historico.push(HistoricoItem {
                        id: doc.id.clone(),
                        acao: Acao::Rejeitado,
                    });
                    rejeitados += 1;
                    persistir!(&doc.id);
                    idx += 1;
                }
                "u" => {
                    db_rejeitar(&mut client, &doc.id, "conteÃƒÂºdo inÃƒÂºtil");
                    let tags_u = prompt_tags(doc, false, &stdin);
                    db_salvar_tags(&mut client, &doc.id, &tags_u);
                    historico.push(HistoricoItem {
                        id: doc.id.clone(),
                        acao: Acao::Inutil,
                    });
                    rejeitados += 1;
                    persistir!(&doc.id);
                    idx += 1;
                }
                "p" => {
                    salvar_pulado_sessao(&doc.id);
                    pulados_sessao.insert(doc.id.clone());
                    historico.push(HistoricoItem {
                        id: doc.id.clone(),
                        acao: Acao::Pulado,
                    });
                    pulados_conta += 1;
                    persistir!(&doc.id);
                    idx += 1;
                }
                "v" => {
                    if let Some(item) = historico.pop() {
                        match &item.acao {
                            Acao::Aprovado => {
                                db_desfazer(&mut client, &item.id);
                                aprovados = aprovados.saturating_sub(1);
                            }
                            Acao::Rejeitado | Acao::Inutil => {
                                db_desfazer(&mut client, &item.id);
                                rejeitados = rejeitados.saturating_sub(1);
                            }
                            Acao::Pulado => {
                                pulados_sessao.remove(&item.id);
                                reescrever_pulados_sessao(&pulados_sessao);
                                pulados_conta = pulados_conta.saturating_sub(1);
                            }
                        }
                        idx = idx.saturating_sub(1);
                        // Persiste estado apÃƒÂ³s desfazer
                        salvar_estado_sessao(&EstadoSessao {
                            started_at: started_at.clone(),
                            aprovados,
                            rejeitados,
                            pulados: pulados_conta,
                            ultimo_documento_id: historico.last().map(|h| h.id.clone()),
                        });
                        println!("  Ã¢Å“â€œ ÃƒÅ¡ltima decisÃƒÂ£o desfeita.");
                        std::thread::sleep(std::time::Duration::from_millis(700));
                    } else {
                        println!("  Nenhuma decisÃƒÂ£o para desfazer.");
                        std::thread::sleep(std::time::Duration::from_millis(700));
                    }
                }
                "h" => {
                    if modo_sugestao == ModoSugestao::Heuristica {
                        modo_sugestao = ModoSugestao::Desligado;
                        println!("  HeurÃƒÂ­stica: DESLIGADA");
                    } else {
                        modo_sugestao = ModoSugestao::Heuristica;
                        println!("  HeurÃƒÂ­stica: LIGADA");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                "t" => {
                    if modo_sugestao == ModoSugestao::IA {
                        modo_sugestao = ModoSugestao::Desligado;
                        println!("  Auto-IA: DESLIGADA");
                    } else {
                        modo_sugestao = ModoSugestao::IA;
                        println!("  Auto-IA: LIGADA");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                "x" => {
                    if modo_sugestao == ModoSugestao::IA {
                        modo_sugestao = ModoSugestao::Desligado;
                        println!("  [IA] Auto-IA desligada.");
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
                "i" => {
                    let s = match &modo_sugestao {
                        ModoSugestao::Heuristica => sugerir_heuristica(doc),
                        _ => sugerir_com_ia(doc),
                    };
                    exibir_sugestao(&s);
                    print!("  [Enter]: ");
                    io::stdout()
                        .flush()
                        .expect("Erro ao exibir prompt pÃƒÂ³s-sugestÃƒÂ£o");
                    let mut buf = String::new();
                    stdin
                        .lock()
                        .read_line(&mut buf)
                        .expect("Erro ao aguardar confirmaÃƒÂ§ÃƒÂ£o de sugestÃƒÂ£o");
                }
                "e" => {
                    exibir_estatisticas(&mut client, inicio_sessao, aprovados, rejeitados, &stdin);
                }
                "?" => {
                    let conteudo = buscar_conteudo_completo(&mut client, &doc.id);
                    exibir_conteudo_completo(&conteudo, &doc.source, &stdin);
                }
                "z" => {
                    config_tui(&mut config, &mut client, &stdin);
                }
                "b" => {
                    browser_tui(&mut client, &stdin);
                }
                "s" => {
                    limpar_tela();
                    println!(
                        "  SessÃƒÂ£o salva. Aprovados={} Rejeitados={} Pulados={}",
                        aprovados, rejeitados, pulados_conta
                    );
                    break 'principal;
                }
                "q" => {
                    limpar_tela();
                    print!("  Sair sem salvar pulados? [s/N]: ");
                    io::stdout()
                        .flush()
                        .expect("Erro ao exibir prompt de saÃƒÂ­da");
                    if ler_linha(&stdin) == "s" {
                        println!(
                            "  Encerrado. Aprovados={} Rejeitados={} Pulados={}",
                            aprovados, rejeitados, pulados_conta
                        );
                        break 'principal;
                    }
                }
                "help" => {
                    exibir_ajuda();
                    print!("  [Enter]: ");
                    io::stdout()
                        .flush()
                        .expect("Erro ao exibir prompt pÃƒÂ³s-ajuda");
                    let mut buf = String::new();
                    stdin
                        .lock()
                        .read_line(&mut buf)
                        .expect("Erro ao aguardar confirmaÃƒÂ§ÃƒÂ£o de ajuda");
                }
                _ => {
                    println!("  Comando '{}' nÃƒÂ£o reconhecido. [h] para ajuda.", decisao);
                    std::thread::sleep(std::time::Duration::from_millis(700));
                }
            }
        }
    }
}























