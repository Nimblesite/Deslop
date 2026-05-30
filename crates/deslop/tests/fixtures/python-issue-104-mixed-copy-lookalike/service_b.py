import codec


def encode_token(raw):
    packed = codec.encode(raw)
    return packed


def decode_token(blob):
    unpacked = codec.decode(blob)
    return unpacked
