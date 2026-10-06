"""
FlyDrop TLS Certificate Generator
Generates self-signed TLS certificates for LocalSend v2 HTTPS protocol
"""

import os
import datetime
import ipaddress
import logging
from cryptography import x509
from cryptography.x509.oid import NameOID
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.primitives import serialization

logger = logging.getLogger("FlyDrop.TLS")


def get_or_generate_cert(cert_dir):
    os.makedirs(cert_dir, exist_ok=True)
    cert_path = os.path.join(cert_dir, "flydrop_cert.pem")
    key_path = os.path.join(cert_dir, "flydrop_key.pem")

    if os.path.exists(cert_path) and os.path.exists(key_path):
        return cert_path, key_path

    try:
        logger.info("Generating new TLS certificate for LocalSend HTTPS...")
        private_key = rsa.generate_private_key(
            public_exponent=65537,
            key_size=2048
        )

        subject = issuer = x509.Name([
            x509.NameAttribute(NameOID.COMMON_NAME, "FlyDrop"),
            x509.NameAttribute(NameOID.ORGANIZATION_NAME, "Pulsar OS"),
        ])

        cert = (
            x509.CertificateBuilder()
            .subject_name(subject)
            .issuer_name(issuer)
            .public_key(private_key.public_key())
            .serial_number(x509.random_serial_number())
            .not_valid_before(datetime.datetime.now(datetime.timezone.utc))
            .not_valid_after(datetime.datetime.now(datetime.timezone.utc) + datetime.timedelta(days=3650))
            .add_extension(
                x509.SubjectAlternativeName([
                    x509.DNSName("localhost"),
                    x509.IPAddress(ipaddress.IPv4Address("127.0.0.1")),
                    x509.IPAddress(ipaddress.IPv4Address("0.0.0.0")),
                ]),
                critical=False,
            )
            .sign(private_key, hashes.SHA256())
        )

        # Write private key
        with open(key_path, "wb") as f:
            f.write(private_key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.TraditionalOpenSSL,
                encryption_algorithm=serialization.NoEncryption()
            ))

        # Write certificate
        with open(cert_path, "wb") as f:
            f.write(cert.public_bytes(serialization.Encoding.PEM))

        logger.info(f"TLS certificate generated at {cert_path}")
        return cert_path, key_path

    except Exception as e:
        logger.error(f"Failed to generate TLS cert: {e}")
        return None, None
