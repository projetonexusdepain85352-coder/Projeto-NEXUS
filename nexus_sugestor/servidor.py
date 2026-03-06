#!/usr/bin/env python3
import json, os, socket, traceback, sys, urllib.request

SOCKET_PATH  = "/tmp/nexus_sugestor.sock"
OLLAMA_URL   = "http://localhost:11434/api/generate"
OLLAMA_MODEL = "mistral"

def avaliar(dominio, conteudo):
    exemplo = '{"util": true, "confianca": 85, "motivo": "razao em uma linha"}'
    prompt = (
        "[INST] Voce e um classificador tecnico. "
        "Responda APENAS com JSON puro, sem markdown, sem explicacao, sem texto adicional. "
        "Avalie se o documento abaixo e util para treinar IA especializada no dominio informado.\n\n"
        f"Dominio: {dominio}\n"
        f"Documento (primeiros 1000 chars):\n{conteudo[:1000]}\n\n"
        f"Responda SOMENTE com este JSON (substitua os valores):\n{exemplo} [/INST]"
    )
    payload = json.dumps({
        "model": OLLAMA_MODEL,
        "prompt": prompt,
        "stream": False,
        "options": {"temperature": 0.1, "num_predict": 60}
    }).encode()

    req = urllib.request.Request(OLLAMA_URL, data=payload,
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        resp = json.loads(r.read())
    texto = resp.get("response", "").strip()
    print(f"[EVAL] resposta bruta: {repr(texto)}", flush=True)

    inicio = texto.find("{")
    fim    = texto.rfind("}") + 1
    if inicio == -1 or fim == 0:
        return {"util": False, "confianca": 0, "motivo": "sem JSON na resposta"}
    try:
        return json.loads(texto[inicio:fim])
    except Exception as e:
        return {"util": False, "confianca": 0, "motivo": f"parse error: {e}"}

def main():
    print("[NEXUS Sugestor] Iniciando servidor Ollama...", flush=True)
    if os.path.exists(SOCKET_PATH):
        os.remove(SOCKET_PATH)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(SOCKET_PATH)
    server.listen(5)
    print("[NEXUS Sugestor] Modelo pronto. Aguardando conexoes.", flush=True)

    while True:
        conn, _ = server.accept()
        try:
            data = b""
            while True:
                chunk = conn.recv(4096)
                if not chunk: break
                data += chunk
                if data.endswith(b"\n"): break

            entrada  = json.loads(data.decode(errors="replace"))
            resultado = avaliar(entrada.get("domain", ""), entrada.get("content", ""))
            resultado.setdefault("util", False)
            resultado.setdefault("confianca", 0)
            resultado.setdefault("motivo", "sem motivo")

            resp = (json.dumps(resultado, ensure_ascii=False) + "\n").encode()
            print(f"[SERVER] enviando: {resp}", flush=True)
            conn.sendall(resp)
        except Exception:
            traceback.print_exc(file=sys.stdout)
            sys.stdout.flush()
            try:
                conn.sendall(b'{"util":false,"confianca":0,"motivo":"erro interno"}\n')
            except:
                pass
        finally:
            conn.close()

if __name__ == "__main__":
    main()
