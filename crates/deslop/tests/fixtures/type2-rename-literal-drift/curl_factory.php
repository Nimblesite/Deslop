<?php

namespace Fixture\Handler;

final class CurlFactory
{
    private static function toCurlVersion(int $value): int
    {
        if ($value === \STREAM_CRYPTO_METHOD_TLSv1_0_CLIENT) {
            return \CURL_SSLVERSION_TLSv1_0;
        }
        if ($value === \STREAM_CRYPTO_METHOD_TLSv1_1_CLIENT) {
            return \CURL_SSLVERSION_TLSv1_1;
        }
        if ($value === \STREAM_CRYPTO_METHOD_TLSv1_2_CLIENT) {
            return \CURL_SSLVERSION_TLSv1_2;
        }
        return \CURL_SSLVERSION_DEFAULT;
    }
}
