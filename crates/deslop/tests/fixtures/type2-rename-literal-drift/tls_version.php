<?php

namespace Fixture\Handler;

final class TlsVersion
{
    public static function toStreamProtocol(int $value): int
    {
        if ($value === \STREAM_CRYPTO_METHOD_TLSv1_0_CLIENT) {
            return \STREAM_CRYPTO_PROTO_TLSv1_0;
        }
        if ($value === \STREAM_CRYPTO_METHOD_TLSv1_1_CLIENT) {
            return \STREAM_CRYPTO_PROTO_TLSv1_1;
        }
        if ($value === \STREAM_CRYPTO_METHOD_TLSv1_2_CLIENT) {
            return \STREAM_CRYPTO_PROTO_TLSv1_2;
        }
        throw new \InvalidArgumentException('unsupported TLS version');
    }
}
