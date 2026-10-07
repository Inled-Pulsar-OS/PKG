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
from concurrent.futures import ThreadPoolExecutor
import requests
import urllib3

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

from .config import Config, CONFIG_DIR
from .tls_cert import get_or_generate_cert

logger = logging.getLogger("FlyDrop.Protocol")

# Subidas simultaneas cuando se envian varios archivos (una peticion HTTP por
# archivo). Una sola peticion por archivo es lo que permite el protocolo
# LocalSend v2; el paralelismo aprovecha el ancho de banda que un unico
# flujo TCP no llega a llenar sobre Wi-Fi.
MAX_PARALLEL_UPLOADS = 4


class HandshakeFailed(Exception):
    """Fallo al abrir el canal TLS: se registra sin volcar traza completa."""


class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def handle_error(self, request, client_address):
        exc = sys.exc_info()[1]
        if isinstance(exc, HandshakeFailed):
            logger.info(f"Canal TLS fallido desde {client_address[0]}: {exc}")
            return
        super().handle_error(request, client_address)


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
        self.dest_paths = {}   # file_id -> destino elegido (los reintentos lo reusan)
        self.file_bytes = {}   # file_id -> bytes del ultimo intento de ese archivo
        self.completed_notified = False
        self.completion_success = None
        self.lock = threading.Lock()

    def update_file_progress(self, file_id, current_total, current_file=""):
        """
        Progreso contado POR ARCHIVO: `current_total` es lo que lleva el intento
        ACTUAL, de modo que un reintento que vuelve a cero corrige el agregado
        en lugar de acumular bytes duplicados (antes un reintento podia marcar
        'progress: 1.55').

        Devuelve True solo cuando toca emitir un evento de progreso (>= 0,15 s):
        antes se lanzaba uno por cada trozo de 64 KB, inundando el main loop con
        miles de GLib.idle_add + senales D-Bus por segundo.
        """
        with self.lock:
            prev = self.file_bytes.get(file_id, 0)
            self.transferred_bytes += current_total - prev
            self.file_bytes[file_id] = current_total
            if current_file:
                self.current_file_name = current_file
            now = time.time()
            dt = now - self.last_update_time
            if dt >= 0.15:
                delta_bytes = self.transferred_bytes - self.last_transferred_bytes
                self.speed = delta_bytes / dt if dt > 0 else 0.0
                self.last_transferred_bytes = self.transferred_bytes
                self.last_update_time = now
                return True
            return False

    def reset_file_progress(self, file_id):
        """Descarta el progreso de un intento fallido de `file_id`."""
        with self.lock:
            prev = self.file_bytes.pop(file_id, 0)
            if prev:
                self.transferred_bytes -= prev


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


class _SharedProgress:
    """
    Contador de progreso compartido entre los hilos de subida (envio paralelo).
    Emite como maximo un evento cada 0,15 s: antes se lanzaba uno por cada
    trozo de 64 KB, y a alta velocidad eso inunda el main loop con miles de
    GLib.idle_add + senales D-Bus por segundo.
    """
    def __init__(self, total_bytes, on_progress):
        self.total_bytes = total_bytes
        self.on_progress = on_progress
        self.transferred = 0
        self.current_file = ""
        self.last_time = time.time()
        self.last_bytes = 0
        self.lock = threading.Lock()

    def add(self, file_name, chunk_len):
        event = None
        with self.lock:
            self.transferred += chunk_len
            self.current_file = file_name
            now = time.time()
            dt = now - self.last_time
            if dt >= 0.15 and self.on_progress:
                speed = (self.transferred - self.last_bytes) / dt if dt > 0 else 0.0
                fraction = (min(1.0, self.transferred / self.total_bytes)
                            if self.total_bytes > 0 else 1.0)
                event = (fraction, speed, file_name, self.transferred, self.total_bytes)
                self.last_bytes = self.transferred
                self.last_time = now
        if event:
            self.on_progress(*event)


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
        self.ssl_ctx = None

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
                self.ssl_ctx = ssl_ctx
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
                # El handshake TLS NO se hace aqui: se negocia por conexion dentro
                # del hilo de cada peticion (ver LocalSendHandler.setup). Asi la
                # cola de accept() nunca se bloquea esperando un handshake y
                # varias conexiones en paralelo no se serializan.
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

    def notify_completed(self, session, success, message):
        """
        Avisa del final de una sesion evitando dobles avisos (varios archivos
        en paralelo), con una excepcion: si un intento fallo y un REINTENTO
        acaba bien, el exito sí se notifica para que la UI no se quede en
        'fallido' con el archivo ya guardado.
        """
        with session.lock:
            recovered = (success and session.completed_notified
                         and session.completion_success is False)
            if session.completed_notified and not recovered:
                return
            session.completed_notified = True
            session.completion_success = success
            session.status = "completed" if success else "failed"
        if self.on_completed:
            self.on_completed(session, success, message)

    def _create_handler(self):
        outer = self

        class LocalSendHandler(BaseHTTPRequestHandler):
            # HTTP/1.1 con keep-alive: una sola conexion por sesion en lugar de
            # un TCP+TLS nuevo por cada peticion/archivo.
            protocol_version = "HTTP/1.1"
            disable_nagle_algorithm = True
            rbufsize = 128 * 1024

            def setup(self):
                # El handshake TLS ocurre aqui, en el hilo de ESTA peticion
                # (wrap_socket transfiere el fd al socket TLS; el objeto crudo
                # queda detach, asi que el cierre posterior no duplica fd).
                if outer.ssl_ctx is not None:
                    try:
                        self.request = outer.ssl_ctx.wrap_socket(self.request, server_side=True)
                    except Exception as e:
                        raise HandshakeFailed(str(e)) from e
                super().setup()

            def finish(self):
                try:
                    super().finish()
                except OSError:
                    pass
                finally:
                    try:
                        self.connection.close()
                    except OSError:
                        pass

            def log_message(self, format, *args):
                logger.info(f"[HTTP] {self.address_string()} - {format % args}")

            def _send_json(self, status_code, data, close=False):
                body = json.dumps(data, ensure_ascii=False).encode("utf-8")
                try:
                    self.send_response(status_code)
                    self.send_header("Content-Type", "application/json; charset=utf-8")
                    self.send_header("Content-Length", str(len(body)))
                    self.send_header("Access-Control-Allow-Origin", "*")
                    if close:
                        self.close_connection = True
                        self.send_header("Connection", "close")
                    self.end_headers()
                    self.wfile.write(body)
                except (BrokenPipeError, ConnectionResetError, OSError) as e:
                    # El remoto desaparecio (habitual con moviles que se duermen
                    # a mitad de envio): no volcar traza, solo cerrar.
                    self.close_connection = True
                    logger.debug(f"Respuesta {status_code} no entregada: {e}")

            def _send_error(self, status_code, message, close=False):
                self._send_json(status_code, {"message": message}, close=close)

            def do_OPTIONS(self):
                self.send_response(200)
                self.send_header("Access-Control-Allow-Origin", "*")
                self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                self.send_header("Access-Control-Allow-Headers", "*")
                self.send_header("Content-Length", "0")
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
                        # El cuerpo del POST no se va a leer: cerrar la conexion
                        # para que no queden bytes huerfanos en keep-alive.
                        self._send_json(404, {"message": "Session not found"}, close=True)
                        return

                    if session.tokens.get(file_id) != token:
                        logger.warning(f"Invalid token for fileId {file_id}")
                        self._send_json(403, {"message": "Invalid file token"}, close=True)
                        return

                    file_meta = session.files_dict.get(file_id, {})
                    file_name = file_meta.get("fileName", f"flydrop_{file_id}")
                    file_name = os.path.basename(file_name)
                    expected_size = int(file_meta.get("size", 0))

                    dest_dir = outer.config.download_dir
                    os.makedirs(dest_dir, exist_ok=True)

                    # Un reintento reutiliza el destino del primer intento: se
                    # sobrescribe en el MISMO archivo en lugar de amontonar
                    # "copia (1).mp4", "(2).mp4"... por cada reinicio del envio.
                    with session.lock:
                        dest_path = session.dest_paths.get(file_id)
                        if not dest_path:
                            base, ext = os.path.splitext(file_name)
                            dest_path = os.path.join(dest_dir, file_name)
                            taken = set(session.dest_paths.values())
                            counter = 1
                            while os.path.exists(dest_path) or dest_path in taken:
                                dest_path = os.path.join(dest_dir, f"{base} ({counter}){ext}")
                                counter += 1
                            session.dest_paths[file_id] = dest_path

                    session.status = "in_progress"

                    is_chunked = self.headers.get("Transfer-Encoding", "").lower() == "chunked"
                    content_len_hdr = self.headers.get("Content-Length")
                    content_length = int(content_len_hdr) if content_len_hdr else expected_size

                    logger.info(f"Receiving file '{file_name}' (expected: {expected_size} bytes, chunked={is_chunked}) to {dest_path}")

                    bytes_received = 0
                    chunk_buffer_size = 128 * 1024
                    complete = False

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
                                        # Consume trailers until empty line: aqui
                                        # acaba el cuerpo de verdad.
                                        while True:
                                            trailer = self.rfile.readline()
                                            if not trailer or trailer in (b"\r\n", b"\n"):
                                                break
                                        complete = True
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
                                        emit = session.update_file_progress(file_id, bytes_received, current_file=file_name)
                                        if emit and outer.on_progress:
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
                                    emit = session.update_file_progress(file_id, bytes_received, current_file=file_name)
                                    if emit and outer.on_progress:
                                        outer.on_progress(session)
                                complete = (remaining == 0 and bytes_received == content_length)

                        if not outer.running:
                            complete = False

                        # Un corte a mitad de camino (FIN sin RST) se daba por
                        # bueno y se guardaba un archivo truncado como exito.
                        if not complete:
                            want = expected_size if is_chunked else content_length
                            raise IOError(f"Transferencia incompleta: {bytes_received} de {want} bytes")

                        if dest_path not in session.saved_files:
                            session.saved_files.append(dest_path)
                        logger.info(f"File '{file_name}' received successfully ({bytes_received} bytes)")

                        if len(session.saved_files) >= len(session.files_dict):
                            outer.notify_completed(session, True, "Transferencia completada")

                        self._send_json(200, {"message": "OK"})

                    except Exception as e:
                        logger.error(f"Error saving incoming file {dest_path}: {e}")
                        # Sin restos: borra el parcial y olvida su progreso para
                        # que el reintento empiece limpio en el mismo archivo.
                        session.status = "failed"
                        session.reset_file_progress(file_id)
                        try:
                            if os.path.exists(dest_path):
                                os.remove(dest_path)
                        except OSError as cleanup_err:
                            logger.warning(f"Could not remove partial file {dest_path}: {cleanup_err}")
                        outer.notify_completed(session, False, str(e))
                        self._send_json(500, {"message": f"Error saving file: {e}"}, close=True)
                    return

                # 4. Cancel Endpoint
                if path in ("/api/localsend/v2/cancel", "/api/localsend/v1/cancel"):
                    session_id = query.get("sessionId", [""])[0]
                    session = outer.active_sessions.get(session_id)
                    if session:
                        outer.notify_completed(session, False, "Transferencia cancelada")
                    self._send_json(200, {"message": "Cancelled"})
                    return

                self._send_error(404, "Endpoint not found", close=True)

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

        # Una sola sesion HTTP para prepare-upload + todos los uploads: con el
        # servidor en HTTP/1.1 keep-alive se reutiliza la conexion (y su TLS) en
        # lugar de abrir un TCP+TLS nuevo por archivo.
        http = requests.Session()
        http.verify = False
        if client_cert:
            http.cert = client_cert
        adapter = requests.adapters.HTTPAdapter(pool_connections=4, pool_maxsize=MAX_PARALLEL_UPLOADS + 4)
        http.mount("http://", adapter)
        http.mount("https://", adapter)

        protocols_to_try = ["https", "http"] if target_protocol == "https" else ["http", "https"]
        used_url_base = None
        session_id = None
        tokens = {}

        try:
            for proto in protocols_to_try:
                url_base = f"{proto}://{target_ip}:{target_port}/api/localsend/v2"
                try:
                    logger.info(f"Connecting to {url_base}/prepare-upload with mTLS cert...")
                    resp = http.post(f"{url_base}/prepare-upload", json=session_meta, timeout=30)
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

            # Progreso compartido entre los hilos de subida (throttled a 0,15 s)
            progress = _SharedProgress(total_bytes, on_progress)
            results = {}
            results_lock = threading.Lock()

            def upload_one(fid, path):
                token = tokens.get(fid)
                if not token:
                    with results_lock:
                        results[fid] = (False, "sin token de subida")
                    return

                fname = os.path.basename(path)
                fsize = os.path.getsize(path)
                upload_url = f"{used_url_base}/upload?sessionId={session_id}&fileId={fid}&token={token}"
                reader = ProgressFileReader(path, lambda n: progress.add(fname, n))
                try:
                    upload_resp = http.post(
                        upload_url,
                        data=reader,
                        headers={
                            "Content-Type": "application/octet-stream",
                            "Content-Length": str(fsize)
                        },
                        timeout=120
                    )
                    ok = upload_resp.status_code == 200
                    err = "" if ok else upload_resp.text
                except Exception as e:
                    ok, err = False, str(e)
                finally:
                    reader.close()

                if not ok:
                    logger.warning(f"Error uploading {fname}: {err}")
                with results_lock:
                    results[fid] = (ok, err)

            pending = list(file_id_map.items())
            if len(pending) <= 1:
                for fid, path in pending:
                    upload_one(fid, path)
            else:
                # Varios archivos en paralelo: LocalSend v2 lo permite (una
                # peticion HTTP independiente por archivo) y evita que N
                # archivos viajen serializados de uno en uno.
                with ThreadPoolExecutor(max_workers=MAX_PARALLEL_UPLOADS,
                                        thread_name_prefix="FlyDrop-Upload") as pool:
                    futures = [pool.submit(upload_one, fid, path) for fid, path in pending]
                    for fut in futures:
                        fut.result()

            failed = [(fid, err) for fid, (ok, err) in results.items() if not ok]
            if failed:
                first_fid, first_err = failed[0]
                fname = os.path.basename(file_id_map.get(first_fid, ""))
                if on_completed:
                    on_completed(False, f"Error al subir {fname}: {first_err or 'error desconocido'} "
                                        f"({len(failed)} de {len(pending)} archivos fallaron)")
                return

            if on_progress:
                on_progress(1.0, 0.0, "Completado", total_bytes, total_bytes)
            if on_completed:
                on_completed(True, "Todos los archivos se enviaron con éxito")
        finally:
            http.close()

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
