"""
FlyDrop LocalSend Protocol v2 Implementation
High-performance HTTPS mTLS & HTTP Server & Client for LocalSend v2 Transfers
Features RFC-compliant HTTP Chunked Transfer Encoding decoder and seekable stream reader.
"""

import os
import sys
import time
import json
import uuid
import ssl
import mimetypes
import logging
import threading
import urllib.parse
from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn
import requests
import urllib3

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

from .config import Config, CONFIG_DIR
from .tls_cert import get_or_generate_cert

logger = logging.getLogger("FlyDrop.Protocol")


class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True
    allow_reuse_address = True


class TransferSession:
    def __init__(self, session_id, sender_info, files_dict, is_incoming=True):
        self.session_id = session_id
        self.sender_info = sender_info
        self.files_dict = files_dict  # {file_id: file_meta}
        self.tokens = {}  # {file_id: token}
        self.is_incoming = is_incoming
        self.status = "waiting"
        self.event = threading.Event()
        self.total_size = sum(int(f.get("size", 0)) for f in files_dict.values())
        self.transferred_bytes = 0
        self.start_time = time.time()
        self.last_update_time = time.time()
        self.last_transferred_bytes = 0
        self.speed = 0.0
        self.saved_files = []
        self.current_file_name = ""
        self.lock = threading.Lock()

    def update_progress(self, added_bytes, current_file=""):
        with self.lock:
            self.transferred_bytes += added_bytes
            if current_file:
                self.current_file_name = current_file
            now = time.time()
            dt = now - self.last_update_time
            if dt >= 0.15:
                delta_bytes = self.transferred_bytes - self.last_transferred_bytes
                self.speed = delta_bytes / dt if dt > 0 else 0.0
                self.last_transferred_bytes = self.transferred_bytes
                self.last_update_time = now


class ProgressFileReader:
    """
    Seekable file reader with progress callbacks for Python requests / urllib3.
    """
    def __init__(self, file_path, on_chunk=None):
        self.file_path = file_path
        self.size = os.path.getsize(file_path)
        self.fp = open(file_path, "rb")
        self.on_chunk = on_chunk

    def read(self, size=-1):
        chunk = self.fp.read(size)
        if chunk and self.on_chunk:
            self.on_chunk(len(chunk))
        return chunk

    def seek(self, offset, whence=0):
        return self.fp.seek(offset, whence)

    def tell(self):
        return self.fp.tell()

    def __len__(self):
        return self.size

    def close(self):
        self.fp.close()


class LocalSendServer:
    def __init__(self, discovery_service, on_transfer_request=None, on_progress=None, on_completed=None):
        self.config = Config.get()
        self.discovery_service = discovery_service
        self.on_transfer_request = on_transfer_request
        self.on_progress = on_progress
        self.on_completed = on_completed
        self.active_sessions = {}
        self.server = None
        self.server_thread = None
        self.running = False
        self.bound_port = self.config.port
        self.use_https = True

    def start(self):
        if self.running:
            return

        cert_path, key_path = get_or_generate_cert(CONFIG_DIR)
        ssl_ctx = None
        if cert_path and key_path and os.path.exists(cert_path) and os.path.exists(key_path):
            try:
                ssl_ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
                ssl_ctx.load_cert_chain(cert_path, key_path)
                ssl_ctx.verify_mode = ssl.CERT_NONE
                self.use_https = True
                self.config.set_value("protocol", "https")
                logger.info("FlyDrop TLS Context initialized for HTTPS (mTLS compatible)")
            except Exception as e:
                logger.warning(f"Could not initialize SSLContext: {e}. Falling back to HTTP.")
                self.use_https = False
                self.config.set_value("protocol", "http")
        else:
            self.use_https = False
            self.config.set_value("protocol", "http")

        ports_to_try = [53317, 53318, 53319, 53320, 0]
        for try_port in ports_to_try:
            try:
                server_address = ("", try_port)
                self.server = ThreadedHTTPServer(server_address, self._create_handler())
                if ssl_ctx:
                    self.server.socket = ssl_ctx.wrap_socket(self.server.socket, server_side=True)
                self.bound_port = self.server.server_port
                self.config.port = self.bound_port
                logger.info(f"FlyDrop LocalSend Server running ({'HTTPS' if self.use_https else 'HTTP'}) on port {self.bound_port}")
                break
            except Exception as e:
                logger.warning(f"Could not bind server to port {try_port}: {e}")
                continue

        if not self.server:
            raise RuntimeError("Failed to bind FlyDrop Server on any available port")

        self.running = True
        self.server_thread = threading.Thread(target=self.server.serve_forever, daemon=True, name="FlyDrop-Server")
        self.server_thread.start()

    def stop(self):
        self.running = False
        if self.server:
            try:
                self.server.shutdown()
                self.server.server_close()
            except Exception:
                pass

    def _create_handler(self):
        outer = self

        class LocalSendHandler(BaseHTTPRequestHandler):
            def log_message(self, format, *args):
                logger.info(f"[HTTP] {self.address_string()} - {format % args}")

            def _send_json(self, status_code, data):
                body = json.dumps(data, ensure_ascii=False).encode("utf-8")
                self.send_response(status_code)
                self.send_header("Content-Type", "application/json; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.send_header("Access-Control-Allow-Origin", "*")
                self.end_headers()
                self.wfile.write(body)

            def _send_error(self, status_code, message):
                self._send_json(status_code, {"message": message})

            def do_OPTIONS(self):
                self.send_response(200)
                self.send_header("Access-Control-Allow-Origin", "*")
                self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                self.send_header("Access-Control-Allow-Headers", "*")
                self.end_headers()

            def do_GET(self):
                parsed = urllib.parse.urlparse(self.path)
                path = parsed.path.rstrip("/")

                if path in ("/api/localsend/v2/info", "/api/localsend/v1/info", "/info"):
                    info = {
                        "alias": outer.config.alias,
                        "version": outer.config.version,
                        "deviceModel": outer.config.device_model,
                        "deviceType": outer.config.device_type,
                        "fingerprint": outer.config.fingerprint,
                        "download": True,
                        "protocol": outer.config.protocol
                    }
                    self._send_json(200, info)
                else:
                    self._send_error(404, "Not Found")

            def do_POST(self):
                parsed = urllib.parse.urlparse(self.path)
                path = parsed.path.rstrip("/")
                query = urllib.parse.parse_qs(parsed.query)

                logger.info(f"Incoming POST request to {path} from {self.client_address[0]}")

                # 1. Register Endpoint
                if path in ("/api/localsend/v2/register", "/api/localsend/v1/register"):
                    content_len = int(self.headers.get("Content-Length", 0))
                    body = self.rfile.read(content_len).decode("utf-8", errors="ignore")
                    try:
                        data = json.loads(body)
                        outer.discovery_service.add_or_update_device(data, self.client_address[0])
                        info = {
                            "alias": outer.config.alias,
                            "version": outer.config.version,
                            "deviceModel": outer.config.device_model,
                            "deviceType": outer.config.device_type,
                            "fingerprint": outer.config.fingerprint,
                            "download": True,
                            "protocol": outer.config.protocol
                        }
                        self._send_json(200, info)
                    except Exception as e:
                        self._send_error(400, f"Invalid JSON: {e}")
                    return

                # 2. Prepare Upload Endpoint
                if path in ("/api/localsend/v2/prepare-upload", "/api/localsend/v1/prepare-upload"):
                    content_len = int(self.headers.get("Content-Length", 0))
                    body = self.rfile.read(content_len).decode("utf-8", errors="ignore")
                    try:
                        req_data = json.loads(body)
                    except Exception as e:
                        logger.error(f"prepare-upload parse error: {e}")
                        self._send_error(400, f"Invalid request body: {e}")
                        return

                    sender_info = req_data.get("info", {})
                    files_dict = req_data.get("files", {})

                    if not files_dict:
                        self._send_error(400, "No files in prepare-upload")
                        return

                    outer.discovery_service.add_or_update_device(sender_info, self.client_address[0])

                    session_id = str(uuid.uuid4())
                    session = TransferSession(session_id, sender_info, files_dict, is_incoming=True)

                    for fid in files_dict.keys():
                        session.tokens[fid] = str(uuid.uuid4())

                    outer.active_sessions[session_id] = session
                    logger.info(f"Created session {session_id} for sender '{sender_info.get('alias')}' ({len(files_dict)} files)")

                    # Auto-Accept Check
                    if outer.config.auto_accept:
                        session.status = "accepted"
                        resp = {
                            "sessionId": session_id,
                            "files": session.tokens
                        }
                        self._send_json(200, resp)
                        if outer.on_transfer_request:
                            outer.on_transfer_request(session, accepted=True)
                        return

                    # Manual User Acceptance
                    user_decision = None

                    def on_user_decision(accepted):
                        nonlocal user_decision
                        user_decision = accepted
                        session.status = "accepted" if accepted else "rejected"
                        session.event.set()

                    if outer.on_transfer_request:
                        outer.on_transfer_request(session, decision_callback=on_user_decision)
                        session.event.wait(timeout=60.0)

                    if session.status == "accepted" or user_decision is True:
                        resp = {
                            "sessionId": session_id,
                            "files": session.tokens
                        }
                        self._send_json(200, resp)
                    else:
                        session.status = "rejected"
                        self._send_error(403, "Transfer declined by recipient")
                    return

                # 3. Upload Endpoint
                if path in ("/api/localsend/v2/upload", "/api/localsend/v1/upload"):
                    session_id = query.get("sessionId", [""])[0]
                    file_id = query.get("fileId", [""])[0]
                    token = query.get("token", [""])[0]

                    session = outer.active_sessions.get(session_id)
                    if not session:
                        logger.warning(f"Upload attempt with unknown sessionId: {session_id}")
                        self._send_error(404, "Session not found")
                        return

                    if session.tokens.get(file_id) != token:
                        logger.warning(f"Invalid token for fileId {file_id}")
                        self._send_error(403, "Invalid file token")
                        return

                    file_meta = session.files_dict.get(file_id, {})
                    file_name = file_meta.get("fileName", f"flydrop_{file_id}")
                    file_name = os.path.basename(file_name)
                    expected_size = int(file_meta.get("size", 0))

                    dest_dir = outer.config.download_dir
                    os.makedirs(dest_dir, exist_ok=True)

                    base, ext = os.path.splitext(file_name)
                    dest_path = os.path.join(dest_dir, file_name)
                    counter = 1
                    while os.path.exists(dest_path):
                        dest_path = os.path.join(dest_dir, f"{base} ({counter}){ext}")
                        counter += 1

                    session.status = "in_progress"

                    is_chunked = self.headers.get("Transfer-Encoding", "").lower() == "chunked"
                    content_len_hdr = self.headers.get("Content-Length")
                    content_length = int(content_len_hdr) if content_len_hdr else expected_size

                    logger.info(f"Receiving file '{file_name}' (expected: {expected_size} bytes, chunked={is_chunked}) to {dest_path}")

                    bytes_received = 0
                    chunk_buffer_size = 64 * 1024

                    try:
                        with open(dest_path, "wb") as f_out:
                            if is_chunked:
                                while outer.running:
                                    line = self.rfile.readline()
                                    if not line:
                                        break
                                    chunk_header = line.split(b";")[0].strip()
                                    if not chunk_header:
                                        continue
                                    try:
                                        chunk_size = int(chunk_header, 16)
                                    except ValueError:
                                        logger.warning(f"Malformed chunk header: {chunk_header}")
                                        break

                                    if chunk_size == 0:
                                        # Consume trailers until empty line
                                        while True:
                                            trailer = self.rfile.readline()
                                            if not trailer or trailer in (b"\r\n", b"\n"):
                                                break
                                        break

                                    rem = chunk_size
                                    while rem > 0 and outer.running:
                                        to_read = min(chunk_buffer_size, rem)
                                        data = self.rfile.read(to_read)
                                        if not data:
                                            break
                                        f_out.write(data)
                                        bytes_received += len(data)
                                        rem -= len(data)
                                        session.update_progress(len(data), current_file=file_name)
                                        if outer.on_progress:
                                            outer.on_progress(session)

                                    # Discard trailing CRLF after chunk data
                                    self.rfile.readline()
                            else:
                                remaining = content_length
                                while remaining > 0 and outer.running:
                                    to_read = min(chunk_buffer_size, remaining)
                                    data = self.rfile.read(to_read)
                                    if not data:
                                        break
                                    f_out.write(data)
                                    bytes_received += len(data)
                                    remaining -= len(data)
                                    session.update_progress(len(data), current_file=file_name)
                                    if outer.on_progress:
                                        outer.on_progress(session)

                        session.saved_files.append(dest_path)
                        logger.info(f"File '{file_name}' received successfully ({bytes_received} bytes)")

                        if len(session.saved_files) >= len(session.files_dict):
                            session.status = "completed"
                            if outer.on_completed:
                                outer.on_completed(session, True, "Transferencia completada")

                        self._send_json(200, {"message": "OK"})

                    except Exception as e:
                        logger.error(f"Error saving incoming file {dest_path}: {e}")
                        session.status = "failed"
                        if outer.on_completed:
                            outer.on_completed(session, False, str(e))
                        self._send_error(500, f"Error saving file: {e}")
                    return

                # 4. Cancel Endpoint
                if path in ("/api/localsend/v2/cancel", "/api/localsend/v1/cancel"):
                    session_id = query.get("sessionId", [""])[0]
                    session = outer.active_sessions.get(session_id)
                    if session:
                        session.status = "cancelled"
                        if outer.on_completed:
                            outer.on_completed(session, False, "Transferencia cancelada")
                    self._send_json(200, {"message": "Cancelled"})
                    return

                self._send_error(404, "Endpoint not found")

        return LocalSendHandler


class LocalSendClient:
    """
    Client for sending files or text to LocalSend v2 devices using mTLS / TLS
    """
    def __init__(self):
        self.config = Config.get()
        self.cert_path, self.key_path = get_or_generate_cert(CONFIG_DIR)

    def _get_client_cert(self):
        if self.cert_path and self.key_path and os.path.exists(self.cert_path) and os.path.exists(self.key_path):
            return (self.cert_path, self.key_path)
        return None

    def send_files_async(self, target_ip, target_port, file_paths, target_protocol="https", on_progress=None, on_completed=None):
        thread = threading.Thread(
            target=self._send_files_worker,
            args=(target_ip, target_port, file_paths, target_protocol, on_progress, on_completed),
            daemon=True,
            name="FlyDrop-Sender"
        )
        thread.start()
        return thread

    def send_text_async(self, target_ip, target_port, text, target_protocol="https", on_progress=None, on_completed=None):
        thread = threading.Thread(
            target=self._send_text_worker,
            args=(target_ip, target_port, text, target_protocol, on_progress, on_completed),
            daemon=True,
            name="FlyDrop-TextSender"
        )
        thread.start()
        return thread

    def _send_files_worker(self, target_ip, target_port, file_paths, target_protocol, on_progress, on_completed):
        expanded_files = []
        for path in file_paths:
            path = os.path.abspath(path)
            if os.path.isfile(path):
                expanded_files.append(path)
            elif os.path.isdir(path):
                for root, _, files in os.walk(path):
                    for file in files:
                        expanded_files.append(os.path.join(root, file))

        if not expanded_files:
            if on_completed:
                on_completed(False, "No hay archivos válidos para enviar")
            return

        files_dict = {}
        file_id_map = {}
        total_bytes = 0

        for path in expanded_files:
            fid = str(uuid.uuid4())
            fname = os.path.basename(path)
            fsize = os.path.getsize(path)
            mime, _ = mimetypes.guess_type(path)
            if not mime:
                mime = "application/octet-stream"

            files_dict[fid] = {
                "id": fid,
                "fileName": fname,
                "size": fsize,
                "fileType": mime
            }
            file_id_map[fid] = path
            total_bytes += fsize

        session_meta = {
            "info": {
                "alias": self.config.alias,
                "version": self.config.version,
                "deviceModel": self.config.device_model,
                "deviceType": self.config.device_type,
                "fingerprint": self.config.fingerprint,
                "port": self.config.port,
                "protocol": self.config.protocol,
                "download": True
            },
            "files": files_dict
        }

        client_cert = self._get_client_cert()
        protocols_to_try = ["https", "http"] if target_protocol == "https" else ["http", "https"]
        prep_data = None
        used_url_base = None
        session_id = None
        tokens = {}

        for proto in protocols_to_try:
            url_base = f"{proto}://{target_ip}:{target_port}/api/localsend/v2"
            try:
                logger.info(f"Connecting to {url_base}/prepare-upload with mTLS cert...")
                resp = requests.post(
                    f"{url_base}/prepare-upload",
                    json=session_meta,
                    cert=client_cert if proto == "https" else None,
                    verify=False,
                    timeout=30
                )
                if resp.status_code == 200:
                    prep_data = resp.json()
                    session_id = prep_data.get("sessionId")
                    tokens = prep_data.get("files", {})
                    used_url_base = url_base
                    break
                elif resp.status_code == 403:
                    if on_completed:
                        on_completed(False, "El destinatario rechazó la transferencia")
                    return
                else:
                    logger.warning(f"prepare-upload failed on {proto} with status {resp.status_code}: {resp.text}")
            except Exception as e:
                logger.warning(f"Connection failed on {proto} to {url_base}: {e}")
                continue

        if not session_id or not used_url_base:
            if on_completed:
                on_completed(False, f"No se pudo establecer conexión con {target_ip}:{target_port}")
            return

        # Stream files
        transferred_total = 0
        start_time = time.time()
        last_time = start_time
        last_bytes = 0

        for fid, path in file_id_map.items():
            token = tokens.get(fid)
            if not token:
                continue

            fname = os.path.basename(path)
            fsize = os.path.getsize(path)
            upload_url = f"{used_url_base}/upload?sessionId={session_id}&fileId={fid}&token={token}"

            def on_chunk_read(chunk_len):
                nonlocal transferred_total, last_time, last_bytes
                transferred_total += chunk_len
                now = time.time()
                dt = now - last_time
                if dt >= 0.15:
                    speed = (transferred_total - last_bytes) / dt if dt > 0 else 0.0
                    last_bytes = transferred_total
                    last_time = now
                    if on_progress:
                        fraction = transferred_total / total_bytes if total_bytes > 0 else 1.0
                        on_progress(fraction, speed, fname, transferred_total, total_bytes)

            try:
                reader = ProgressFileReader(path, on_chunk_read)
                headers = {
                    "Content-Type": "application/octet-stream",
                    "Content-Length": str(fsize)
                }
                upload_resp = requests.post(
                    upload_url,
                    data=reader,
                    headers=headers,
                    cert=client_cert if used_url_base.startswith("https") else None,
                    verify=False,
                    timeout=120
                )
                reader.close()
                if upload_resp.status_code != 200:
                    if on_completed:
                        on_completed(False, f"Error al subir {fname}: {upload_resp.text}")
                    return
            except Exception as e:
                if on_completed:
                    on_completed(False, f"Error transfiriendo {fname}: {e}")
                return

        if on_progress:
            on_progress(1.0, 0.0, "Completado", total_bytes, total_bytes)
        if on_completed:
            on_completed(True, "Todos los archivos se enviaron con éxito")

    def _send_text_worker(self, target_ip, target_port, text, target_protocol, on_progress, on_completed):
        fid = str(uuid.uuid4())
        text_bytes = text.encode("utf-8")
        fsize = len(text_bytes)

        files_dict = {
            fid: {
                "id": fid,
                "fileName": "texto.txt",
                "size": fsize,
                "fileType": "text/plain",
                "preview": text[:100]
            }
        }

        session_meta = {
            "info": {
                "alias": self.config.alias,
                "version": self.config.version,
                "deviceModel": self.config.device_model,
                "deviceType": self.config.device_type,
                "fingerprint": self.config.fingerprint,
                "port": self.config.port,
                "protocol": self.config.protocol,
                "download": True
            },
            "files": files_dict
        }

        client_cert = self._get_client_cert()
        protocols_to_try = ["https", "http"] if target_protocol == "https" else ["http", "https"]
        prep_data = None
        used_url_base = None
        session_id = None
        tokens = {}

        for proto in protocols_to_try:
            url_base = f"{proto}://{target_ip}:{target_port}/api/localsend/v2"
            try:
                resp = requests.post(
                    f"{url_base}/prepare-upload",
                    json=session_meta,
                    cert=client_cert if proto == "https" else None,
                    verify=False,
                    timeout=30
                )
                if resp.status_code == 200:
                    prep_data = resp.json()
                    session_id = prep_data.get("sessionId")
                    tokens = prep_data.get("files", {})
                    used_url_base = url_base
                    break
                elif resp.status_code == 403:
                    if on_completed:
                        on_completed(False, "El destinatario rechazó la transferencia")
                    return
            except Exception as e:
                logger.warning(f"Connection failed on {proto} to {url_base}: {e}")
                continue

        if not session_id or not used_url_base:
            if on_completed:
                on_completed(False, f"No se pudo establecer conexión con {target_ip}:{target_port}")
            return

        token = tokens.get(fid)
        upload_url = f"{used_url_base}/upload?sessionId={session_id}&fileId={fid}&token={token}"

        try:
            upload_resp = requests.post(
                upload_url,
                data=text_bytes,
                headers={"Content-Type": "text/plain; charset=utf-8", "Content-Length": str(fsize)},
                cert=client_cert if used_url_base.startswith("https") else None,
                verify=False,
                timeout=60
            )
            if upload_resp.status_code != 200:
                if on_completed:
                    on_completed(False, f"Error enviando texto: {upload_resp.text}")
                return
        except Exception as e:
            if on_completed:
                on_completed(False, f"Error transfiriendo texto: {e}")
            return

        if on_progress:
            on_progress(1.0, 0.0, "Completado", fsize, fsize)
        if on_completed:
            on_completed(True, "Texto enviado con éxito")
