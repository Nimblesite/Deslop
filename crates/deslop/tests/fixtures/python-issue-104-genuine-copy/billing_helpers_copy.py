import base64


def checksum(raw):
    digest = base64.b64encode(raw)
    return digest


def verify(blob):
    decoded = base64.b64decode(blob)
    return decoded
