-- Seed 001: benchmark_questions (10 por dominio)
BEGIN;

INSERT INTO benchmark_questions (domain, question, expected_answer, expected_keywords, created_by)
VALUES
-- INFRA
('infra', 'No Dockerfile, qual a diferenca entre CMD e ENTRYPOINT?', 'ENTRYPOINT define o executavel principal do container; CMD define o comando ou argumentos padrao que podem ser sobrescritos.', ARRAY['dockerfile','entrypoint','cmd','default'], 'nexus_mtp_seed'),
('infra', 'O que a instrucao EXPOSE faz em uma imagem Docker?', 'EXPOSE documenta a porta usada pelo container; ela nao publica a porta sozinha.', ARRAY['expose','port','publish','container'], 'nexus_mtp_seed'),
('infra', 'Quais mecanismos base de isolamento sao usados por containers Linux?', 'Containers usam namespaces para isolamento e cgroups para controle de recursos.', ARRAY['namespaces','cgroups','isolation','resources'], 'nexus_mtp_seed'),
('infra', 'Quais secoes aparecem com frequencia em um unit file do systemd?', 'Um unit file normalmente define secoes como [Unit], [Service] e [Install].', ARRAY['unit file','[unit]','[service]','[install]'], 'nexus_mtp_seed'),
('infra', 'O que o comando systemctl enable faz para um servico?', 'systemctl enable cria os vinculos para iniciar o servico automaticamente no boot.', ARRAY['systemctl','enable','boot','service'], 'nexus_mtp_seed'),
('infra', 'Para que serve o journalctl no ecossistema systemd?', 'journalctl consulta e filtra logs do journal gerenciado pelo systemd.', ARRAY['journalctl','logs','systemd','journal'], 'nexus_mtp_seed'),
('infra', 'Qual arquivo do PostgreSQL controla regras de autenticacao de clientes?', 'As regras de autenticacao por host no PostgreSQL ficam no arquivo pg_hba.conf.', ARRAY['pg_hba.conf','authentication','client','host'], 'nexus_mtp_seed'),
('infra', 'Qual funcao SQL recarrega configuracoes no PostgreSQL sem reiniciar?', 'A funcao pg_reload_conf() recarrega configuracoes sem restart completo.', ARRAY['pg_reload_conf','reload','configuration','postgresql'], 'nexus_mtp_seed'),
('infra', 'Qual e o objetivo principal do VACUUM no PostgreSQL?', 'VACUUM remove tuplas mortas (dead tuples) e ajuda a recuperar/otimizar armazenamento.', ARRAY['vacuum','dead','tuple','storage'], 'nexus_mtp_seed'),
('infra', 'Quais comandos sao usados para carregar modulos do kernel em runtime?', 'modprobe e insmod sao comandos usados para carregar modulos do kernel Linux.', ARRAY['kernel','module','modprobe','load'], 'nexus_mtp_seed'),

-- RUST
('rust', 'Como ownership funciona em Rust em termos de ciclo de vida de valores?', 'Cada valor tem um owner; quando o owner sai de escopo, o valor e dropado.', ARRAY['ownership','owner','scope','drop'], 'nexus_mtp_seed'),
('rust', 'Qual regra de borrowing envolve referencias mutaveis e imutaveis?', 'Rust permite varias referencias imutaveis ou uma mutavel por vez para evitar conflitos.', ARRAY['mutable','immutable','references','borrow'], 'nexus_mtp_seed'),
('rust', 'Qual problema as lifetimes ajudam a prevenir?', 'Lifetimes garantem que referencias permanecam validas e evitam dangling references.', ARRAY['lifetime','references','dangling','valid'], 'nexus_mtp_seed'),
('rust', 'O que significa uma referencia com lifetime ''static em Rust?', 'Uma referencia ''static e valida por toda a duracao do programa.', ARRAY['''static','lifetime','program','duration'], 'nexus_mtp_seed'),
('rust', 'O que uma async fn retorna conceitualmente em Rust?', 'Uma async fn retorna um Future que sera executado/aguardado com await.', ARRAY['async fn','future','returns','await'], 'nexus_mtp_seed'),
('rust', 'Qual o papel do Tokio no ecossistema async de Rust?', 'Tokio e um runtime amplamente usado para executar tarefas e I/O assincronos em Rust.', ARRAY['tokio','runtime','async','tasks'], 'nexus_mtp_seed'),
('rust', 'Quando o bloco unsafe e necessario em Rust?', 'unsafe e necessario para operacoes como dereferenciar raw pointer e chamar unsafe function.', ARRAY['unsafe','raw pointer','unsafe function','safety'], 'nexus_mtp_seed'),
('rust', 'Para que servem traits na linguagem Rust?', 'Traits definem comportamento compartilhado que tipos podem implementar.', ARRAY['trait','behavior','types','implementation'], 'nexus_mtp_seed'),
('rust', 'Como trait objects com dyn funcionam em dispatch?', 'Trait object com dyn usa dynamic dispatch em runtime para chamar implementacoes.', ARRAY['trait object','dyn','dynamic dispatch','runtime'], 'nexus_mtp_seed'),
('rust', 'Qual a ideia de usar impl Trait em tipo de retorno?', 'impl Trait no retorno abstrai o tipo concreto, exigindo apenas que implemente a trait.', ARRAY['impl trait','return type','concrete type','abstraction'], 'nexus_mtp_seed'),

-- MLOPS
('mlops', 'Qual e a ideia central do LoRA no fine-tuning?', 'LoRA adiciona matrizes low-rank como adapters treinaveis sem atualizar todos os pesos do modelo.', ARRAY['lora','low-rank','adapter','trainable'], 'nexus_mtp_seed'),
('mlops', 'Como o QLoRA combina quantizacao e adapters?', 'QLoRA aplica quantization 4-bit no modelo base e treina adapters LoRA por cima.', ARRAY['qlora','4-bit','quantization','lora'], 'nexus_mtp_seed'),
('mlops', 'O que significa NF4 no contexto de quantizacao?', 'NF4 e um formato de quantization 4-bit usado com bitsandbytes para reduzir memoria.', ARRAY['nf4','4-bit','bitsandbytes','quantization'], 'nexus_mtp_seed'),
('mlops', 'Qual objetivo do PEFT no ajuste de modelos?', 'PEFT reduz custo treinando poucos trainable parameters enquanto o base model permanece frozen.', ARRAY['peft','trainable parameters','base model','frozen'], 'nexus_mtp_seed'),
('mlops', 'O que a opcao load_in_4bit busca no pipeline de treino?', 'load_in_4bit carrega pesos em 4-bit para reduzir uso de memoria via quantization.', ARRAY['load_in_4bit','4-bit','memory','quantization'], 'nexus_mtp_seed'),
('mlops', 'O que caracteriza supervised fine-tuning?', 'Supervised fine-tuning usa pares input-output como sinal supervisionado para ajustar o modelo.', ARRAY['supervised fine-tuning','input','output','pairs'], 'nexus_mtp_seed'),
('mlops', 'Como rank e alpha influenciam adapters LoRA?', 'rank define capacidade do adapter e alpha controla a escala aplicada na atualizacao LoRA.', ARRAY['rank','alpha','lora','adapter'], 'nexus_mtp_seed'),
('mlops', 'Quais modulos sao alvos comuns de LoRA em arquiteturas transformer?', 'Configuracoes comuns aplicam LoRA em target modules como q_proj, k_proj, v_proj e o_proj.', ARRAY['target modules','q_proj','k_proj','v_proj'], 'nexus_mtp_seed'),
('mlops', 'Por que adapters sao usados em vez de ajustar todos os pesos?', 'Adapters reduzem memoria e quantidade de trainable parameters comparado ao full fine-tuning.', ARRAY['adapters','memory','trainable parameters','fine-tuning'], 'nexus_mtp_seed'),
('mlops', 'Para que serve gradient accumulation em treino com batch pequeno?', 'Gradient accumulation simula batch efetivo maior acumulando gradientes antes do update.', ARRAY['gradient accumulation','batch','gradients','update'], 'nexus_mtp_seed'),

-- SECURITY
('security', 'Qual mudanca de latencia no handshake do TLS 1.3 e frequentemente destacada?', 'TLS 1.3 reduz latencia de handshake para 1-RTT em comparacao a versoes anteriores.', ARRAY['tls 1.3','handshake','1-rtt','protocol'], 'nexus_mtp_seed'),
('security', 'Qual RFC define a especificacao do TLS 1.3?', 'A especificacao do TLS 1.3 esta no RFC 8446.', ARRAY['rfc 8446','tls 1.3','protocol','handshake'], 'nexus_mtp_seed'),
('security', 'Qual o objetivo do OWASP Top 10 para aplicacoes web?', 'OWASP Top 10 resume os principais riscos de seguranca web para orientar mitigacoes.', ARRAY['owasp top 10','risks','web','security'], 'nexus_mtp_seed'),
('security', 'O que representa a categoria Broken Access Control no OWASP Top 10?', 'Broken Access Control cobre falhas de autorizacao e controle de acesso indevido.', ARRAY['broken access control','owasp top 10','authorization','access'], 'nexus_mtp_seed'),
('security', 'Qual tipo de falha e coberto pela categoria Injection?', 'Injection cobre casos em que entrada nao confiavel altera comandos/queries executadas.', ARRAY['injection','untrusted','input','queries'], 'nexus_mtp_seed'),
('security', 'O que Security Misconfiguration enfatiza em praticas defensivas?', 'Security Misconfiguration destaca risco de configuracao insegura e necessidade de hardening.', ARRAY['security misconfiguration','configuration','default','hardening'], 'nexus_mtp_seed'),
('security', 'Para que serve um identificador CVE?', 'CVE fornece um identifier unico para rastrear uma vulnerability conhecida.', ARRAY['cve-','identifier','vulnerability','nvd'], 'nexus_mtp_seed'),
('security', 'Como o CVSS e usado na triagem de vulnerabilidades?', 'CVSS fornece score de severidade para priorizar tratamento de vulnerabilities.', ARRAY['cvss','score','severity','vulnerability'], 'nexus_mtp_seed'),
('security', 'Quais funcoes centrais aparecem no NIST Cybersecurity Framework?', 'O NIST Cybersecurity Framework organiza atividades em Identify, Protect, Detect, Respond e Recover.', ARRAY['identify','protect','detect','respond','recover'], 'nexus_mtp_seed'),
('security', 'Como o NIST Cybersecurity Framework se relaciona com gestao de risco?', 'O framework orienta outcomes de seguranca para tratar risk e threat de forma estruturada.', ARRAY['nist cybersecurity framework','outcomes','risk','threat'], 'nexus_mtp_seed')
ON CONFLICT (domain, question)
DO UPDATE
SET
    expected_answer = EXCLUDED.expected_answer,
    expected_keywords = EXCLUDED.expected_keywords,
    created_by = EXCLUDED.created_by;

COMMIT;