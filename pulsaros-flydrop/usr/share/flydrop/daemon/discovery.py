"""
FlyDrop Discovery Service
Implements LocalSend v2 UDP Multicast Announcement & Continuous Background Discovery
"""

import socket
import struct
import json
import time
import threading
import logging
from .config import Config

logger = logging.getLogger("FlyDrop.Discovery")

MULTICAST_GROUP_IPV4 = "224.0.0.167"
MULTICAST_PORT = 53317


class DeviceDiscovery:
    def __init__(self, on_device_found=None, on_device_lost=None):
        self.config = Config.get()
        self.on_device_found = on_device_found
        self.on_device_lost = on_device_lost
        self.devices = {}  # key: fingerprint or ip:port -> device dict
        self.running = False
        self._rx_socket = None
        self._tx_socket = None
        self._rx_thread = None
        self._scan_thread = None
        self._lock = threading.Lock()

    def start(self):
        if self.running:
            return
        self.running = True

        # Socket for sending multicast / broadcast packets
        try:
            self._tx_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
            self._tx_socket.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 4)
            self._tx_socket.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
        except Exception as e:
            logger.error(f"Error creating TX discovery socket: {e}")

        # Socket for receiving multicast packets
        try:
            self._rx_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
            self._rx_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            if hasattr(socket, "SO_REUSEPORT"):
                try:
                    self._rx_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEPORT, 1)
                except Exception:
                    pass

            self._rx_socket.bind(("", MULTICAST_PORT))

            # Join multicast group on all local interfaces
            mreq = struct.pack("4sl", socket.inet_aton(MULTICAST_GROUP_IPV4), socket.INADDR_ANY)
            try:
                self._rx_socket.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)
            except Exception as e:
                logger.debug(f"IP_ADD_MEMBERSHIP default error: {e}")

            for ip, _ in self._get_network_interfaces():
                try:
                    mreq_if = struct.pack("4s4s", socket.inet_aton(MULTICAST_GROUP_IPV4), socket.inet_aton(ip))
                    self._rx_socket.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq_if)
                except Exception:
                    pass

            logger.info("Bound discovery RX socket to 224.0.0.167:53317")
        except Exception as e:
            logger.warning(f"Could not bind to standard discovery port {MULTICAST_PORT}: {e}")
            try:
                self._rx_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
                self._rx_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                self._rx_socket.bind(("", 0))
            except Exception as e2:
                logger.error(f"Failed to create secondary RX socket: {e2}")

        if self._rx_socket:
            self._rx_thread = threading.Thread(target=self._listen_loop, daemon=True, name="FlyDrop-Discovery-RX")
            self._rx_thread.start()

        self._scan_thread = threading.Thread(target=self._continuous_scan_loop, daemon=True, name="FlyDrop-Discovery-Scan")
        self._scan_thread.start()

        # Send initial query and announcement
        self.send_announcement(is_announcement=False)
        self.send_announcement(is_announcement=True)

    def _get_network_interfaces(self):
        """Returns list of (ip, broadcast_ip) for all active non-loopback IPv4 interfaces"""
        results = []
        try:
            import subprocess
            out = subprocess.check_output(['ip', '-o', '-4', 'addr', 'show'], text=True, timeout=2)
            for line in out.strip().split('\n'):
                parts = line.split()
                if len(parts) >= 4 and parts[1] != 'lo':
                    ip_with_mask = parts[3]
                    ip = ip_with_mask.split('/')[0]
                    brd = None
                    if 'brd' in parts:
                        brd = parts[parts.index('brd') + 1]
                    results.append((ip, brd))
        except Exception:
            pass
        return results

    def stop(self):
        self.running = False
        if self._rx_socket:
            try:
                self._rx_socket.close()
            except Exception:
                pass
        if self._tx_socket:
            try:
                self._tx_socket.close()
            except Exception:
                pass

    def send_announcement(self, is_announcement=True, target_ip=None, target_port=None):
        """
        Sends LocalSend v2 announcement packet.
        If is_announcement is True, announces presence.
        If False, queries the network for other devices.
        """
        msg = {
            "alias": self.config.alias,
            "version": self.config.version,
            "deviceModel": self.config.device_model,
            "deviceType": self.config.device_type,
            "fingerprint": self.config.fingerprint,
            "port": self.config.port,
            "protocol": self.config.protocol,
            "download": True,
            "announcement": bool(is_announcement),
            "announce": bool(is_announcement)
        }
        data = json.dumps(msg).encode("utf-8")

        if not self._tx_socket:
            return

        try:
            if target_ip and target_port:
                self._tx_socket.sendto(data, (target_ip, int(target_port)))
            else:
                # Send to multicast group
                try:
                    self._tx_socket.sendto(data, (MULTICAST_GROUP_IPV4, MULTICAST_PORT))
                except Exception:
                    pass

                # Send to global broadcast
                try:
                    self._tx_socket.sendto(data, ("255.255.255.255", MULTICAST_PORT))
                except Exception:
                    pass

                # Send to each interface's subnet broadcast
                for _, brd in self._get_network_interfaces():
                    if brd:
                        try:
                            self._tx_socket.sendto(data, (brd, MULTICAST_PORT))
                        except Exception:
                            pass
        except Exception as e:
            logger.debug(f"Error sending discovery packet: {e}")

    def _listen_loop(self):
        while self.running:
            try:
                data, addr = self._rx_socket.recvfrom(65535)
                if not data:
                    continue
                sender_ip = addr[0]
                text = data.decode("utf-8", errors="ignore")
                msg = json.loads(text)

                fingerprint = msg.get("fingerprint")
                if not fingerprint or fingerprint == self.config.fingerprint:
                    continue

                port = int(msg.get("port", 53317))
                device = {
                    "fingerprint": fingerprint,
                    "alias": msg.get("alias", "Dispositivo LocalSend"),
                    "deviceModel": msg.get("deviceModel", "LocalSend"),
                    "deviceType": msg.get("deviceType", "desktop"),
                    "ip": sender_ip,
                    "port": port,
                    "protocol": msg.get("protocol", "https"),
                    "download": msg.get("download", True),
                    "version": msg.get("version", "2.1"),
                    "last_seen": time.time()
                }

                with self._lock:
                    self.devices[fingerprint] = device

                if self.on_device_found:
                    try:
                        self.on_device_found(device)
                    except Exception as e:
                        logger.error(f"Error in on_device_found: {e}")

                # If the packet was a scan query, reply directly to the sender
                is_query = not msg.get("announcement", True) or not msg.get("announce", True)
                if is_query:
                    self.send_announcement(is_announcement=True, target_ip=sender_ip, target_port=port)

            except Exception as e:
                if self.running:
                    time.sleep(0.1)

    def _continuous_scan_loop(self):
        """Actively scans the LAN in the background every 3 seconds"""
        while self.running:
            now = time.time()
            # Send discovery query to locate all active LocalSend devices
            self.send_announcement(is_announcement=False)

            # Clean stale devices (inactive for > 45 seconds)
            with self._lock:
                to_remove = []
                for fp, dev in self.devices.items():
                    if now - dev.get("last_seen", 0) > 45:
                        to_remove.append(fp)
                for fp in to_remove:
                    dev = self.devices.pop(fp, None)
                    if dev and self.on_device_lost:
                        try:
                            self.on_device_lost(dev)
                        except Exception:
                            pass

            time.sleep(3.0)

    def get_devices(self):
        with self._lock:
            return list(self.devices.values())

    def add_or_update_device(self, device_data, ip):
        fingerprint = device_data.get("fingerprint")
        if not fingerprint or fingerprint == self.config.fingerprint:
            return None
        port = int(device_data.get("port", 53317))
        device = {
            "fingerprint": fingerprint,
            "alias": device_data.get("alias", "Dispositivo"),
            "deviceModel": device_data.get("deviceModel", "LocalSend"),
            "deviceType": device_data.get("deviceType", "desktop"),
            "ip": ip,
            "port": port,
            "protocol": device_data.get("protocol", "https"),
            "download": device_data.get("download", True),
            "version": device_data.get("version", "2.1"),
            "last_seen": time.time()
        }
        with self._lock:
            self.devices[fingerprint] = device
        if self.on_device_found:
            self.on_device_found(device)
        return device
