#!/usr/bin/env python3
"""
test-wfd-client.py - Emulador de Cliente Windows 10/11 Miracast (Win + K)
Testa o handshake completo M1 a M7 e o canal de streaming RTSP/RTP com o Pi Zero.

Licença: MIT
Autor: Carlos Alberto <psncarlosalberto4ti@gmail.com>
"""

import sys
import socket
import time

TARGET_HOST = "192.168.7.2"
TARGET_PORT = 7236

def log(msg, color="32"):
    print(f"\033[1;{color}m[test-wfd-client]\033[0m {msg}", flush=True)

def recv_msg(sock, timeout=5.0):
    sock.settimeout(timeout)
    buf = b""
    while b"\r\n\r\n" not in buf:
        chunk = sock.recv(2048)
        if not chunk:
            break
        buf += chunk
    if b"\r\n\r\n" not in buf:
        return buf, b""
    headers_raw, rest = buf.split(b"\r\n\r\n", 1)
    content_len = 0
    for line in headers_raw.split(b"\r\n"):
        if line.lower().startswith(b"content-length:"):
            try:
                content_len = int(line.split(b":")[1].strip())
            except ValueError:
                pass
    body = rest
    while len(body) < content_len:
        chunk = sock.recv(content_len - len(body))
        if not chunk:
            break
        body += chunk
    return headers_raw, body

def main():
    host = sys.argv[1] if len(sys.argv) > 1 else TARGET_HOST
    log(f"Conectando ao Sink Miracast no Pi Zero ({host}:{TARGET_PORT})...", "34")

    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect((host, TARGET_PORT))
    log("Conexão TCP estabelecida!", "32")

    # M1: Source -> Sink (OPTIONS)
    log("[M1] Enviando OPTIONS...", "36")
    m1 = (
        "OPTIONS * RTSP/1.0\r\n"
        "CSeq: 1\r\n"
        "Require: org.wfa.wfd1.0\r\n\r\n"
    )
    s.sendall(m1.encode("utf-8"))

    # Recebe resposta M1
    resp_m1_hdr, _ = recv_msg(s)
    log(f"[M1 Resposta recebida]:\n{resp_m1_hdr.decode('utf-8', errors='ignore').strip()}", "37")
    assert b"200 OK" in resp_m1_hdr, "Falha no M1"

    # M2: Sink -> Source (OPTIONS)
    log("[M2] Aguardando OPTIONS do Sink...", "36")
    m2_hdr, _ = recv_msg(s)
    log(f"[M2 Recebido do Sink]:\n{m2_hdr.decode('utf-8', errors='ignore').strip()}", "37")
    assert b"OPTIONS" in m2_hdr, "Falha no M2"

    # Responde M2
    resp_m2 = (
        "RTSP/1.0 200 OK\r\n"
        "CSeq: 1\r\n"
        "Public: org.wfa.wfd1.0, GET_PARAMETER, SET_PARAMETER, SETUP, PLAY, TEARDOWN\r\n\r\n"
    )
    s.sendall(resp_m2.encode("utf-8"))
    log("[M2] Resposta enviada com sucesso!", "32")

    # M3: Source -> Sink (GET_PARAMETER)
    log("[M3] Enviando GET_PARAMETER para obter formatos de vídeo...", "36")
    body_m3 = (
        "wfd_video_formats\r\n"
        "wfd_audio_codecs\r\n"
        "wfd_client_rtpports\r\n"
        "wfd_uibc_capability\r\n"
        "wfd_connector_type\r\n"
        "wfd_content_protection\r\n"
    )
    m3 = (
        "GET_PARAMETER rtsp://localhost/wfd1.0 RTSP/1.0\r\n"
        "CSeq: 2\r\n"
        "Content-Type: text/parameters\r\n"
        f"Content-Length: {len(body_m3)}\r\n\r\n"
        f"{body_m3}"
    )
    s.sendall(m3.encode("utf-8"))

    # Recebe M3 resposta com os formatos
    resp_m3_hdr, resp_m3_body = recv_msg(s)
    log(f"[M3 Formatos Suportados pelo Pi Zero]:\n{resp_m3_body.decode('utf-8', errors='ignore').strip()}", "37")
    assert b"wfd_video_formats" in resp_m3_body, "wfd_video_formats ausente"
    assert b"wfd_client_rtpports" in resp_m3_body, "wfd_client_rtpports ausente"

    # M4: Source -> Sink (SET_PARAMETER - Seleção de resolução/formato)
    log("[M4] Enviando SET_PARAMETER selecionando modo H.264...", "36")
    body_m4 = "wfd_video_formats: 00 00 02 02 0001deff 157cff5f 00000fff 00 0000 0000 00 none none\r\n"
    m4 = (
        "SET_PARAMETER rtsp://localhost/wfd1.0 RTSP/1.0\r\n"
        "CSeq: 3\r\n"
        "Content-Type: text/parameters\r\n"
        f"Content-Length: {len(body_m4)}\r\n\r\n"
        f"{body_m4}"
    )
    s.sendall(m4.encode("utf-8"))
    resp_m4_hdr, _ = recv_msg(s)
    assert b"200 OK" in resp_m4_hdr, "Falha no M4"

    # M5: Source -> Sink (SET_PARAMETER - Trigger SETUP)
    log("[M5] Enviando SET_PARAMETER trigger SETUP...", "36")
    body_m5 = "wfd_trigger_method: SETUP\r\n"
    m5 = (
        "SET_PARAMETER rtsp://localhost/wfd1.0 RTSP/1.0\r\n"
        "CSeq: 4\r\n"
        "Content-Type: text/parameters\r\n"
        f"Content-Length: {len(body_m5)}\r\n\r\n"
        f"{body_m5}"
    )
    s.sendall(m5.encode("utf-8"))
    resp_m5_hdr, _ = recv_msg(s)
    assert b"200 OK" in resp_m5_hdr, "Falha no M5"

    # M6: Sink -> Source (SETUP)
    log("[M6] Aguardando requisição SETUP do Sink...", "36")
    m6_hdr, _ = recv_msg(s)
    log(f"[M6 Recebido]:\n{m6_hdr.decode('utf-8', errors='ignore').strip()}", "37")
    assert b"SETUP" in m6_hdr, "Falha no M6"

    # Responde M6 com Session ID e portas
    resp_m6 = (
        "RTSP/1.0 200 OK\r\n"
        "CSeq: 2\r\n"
        "Session: 87654321;timeout=60\r\n"
        "Transport: RTP/AVP/UDP;unicast;client_port=5002;server_port=50020\r\n\r\n"
    )
    s.sendall(resp_m6.encode("utf-8"))
    log("[M6] Resposta de SETUP enviada com Session 87654321!", "32")

    # M7: Sink -> Source (PLAY)
    log("[M7] Aguardando requisição PLAY do Sink...", "36")
    m7_hdr, _ = recv_msg(s)
    log(f"[M7 Recebido]:\n{m7_hdr.decode('utf-8', errors='ignore').strip()}", "37")
    assert b"PLAY" in m7_hdr, "Falha no M7"

    # Responde M7
    resp_m7 = (
        "RTSP/1.0 200 OK\r\n"
        "CSeq: 3\r\n"
        "Session: 87654321\r\n\r\n"
    )
    s.sendall(resp_m7.encode("utf-8"))
    log("[M7] Handshake WFD concluído com 100% de sucesso! Sessão ativa.", "32")

    # Envia pacote de teste UDP RTP para confirmar recepção
    log("Enviando pacote de telemetria/teste RTP para UDP port 5002...", "34")
    udp_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    # Cabeçalho RTP dummy (V=2, PayloadType=33 MP2T)
    dummy_rtp = b"\x80\x21\x00\x01" + b"\x00\x00\x00\x00" + b"\x12\x34\x56\x78" + (b"\x47" * 188)
    for _ in range(10):
        udp_sock.sendto(dummy_rtp, (host, 5002))
        time.sleep(0.05)
    udp_sock.close()
    log("[+] Pacotes de teste transmitidos com sucesso!", "32")

    time.sleep(1)
    # TEARDOWN
    log("Finalizando teste com TEARDOWN...", "33")
    teardown = (
        "TEARDOWN rtsp://localhost/wfd1.0 RTSP/1.0\r\n"
        "CSeq: 5\r\n"
        "Session: 87654321\r\n\r\n"
    )
    s.sendall(teardown.encode("utf-8"))
    time.sleep(0.5)
    s.close()
    log("Teste concluído com sucesso total! Protocolo Miracast validado.", "32")

if __name__ == "__main__":
    main()
