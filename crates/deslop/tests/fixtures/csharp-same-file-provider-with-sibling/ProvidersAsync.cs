namespace CacheDemo
{
    public class SerializingProviderAsync<TStored>
    {
        private readonly IAsyncStore<TStored> _store;
        private readonly ICodec<object, TStored> _codec;

        public SerializingProviderAsync(IAsyncStore<TStored> store, ICodec<object, TStored> codec)
        {
            if (store == null) throw new ArgumentNullException(nameof(store));
            if (codec == null) throw new ArgumentNullException(nameof(codec));
            _store = store;
            _codec = codec;
        }

        public async Task<object> ReadAsync(string key, CancellationToken token)
        {
            return _codec.Decode(await _store.ReadAsync(key, token));
        }

        public async Task WriteAsync(string key, object value, TimeSpan lifetime, CancellationToken token)
        {
            await _store.WriteAsync(key, _codec.Encode(value), lifetime, token);
        }
    }

    public class SerializingProviderAsync<TValue, TStored>
    {
        private readonly IAsyncStore<TStored> _store;
        private readonly ICodec<TValue, TStored> _codec;

        public SerializingProviderAsync(IAsyncStore<TStored> store, ICodec<TValue, TStored> codec)
        {
            if (store == null) throw new ArgumentNullException(nameof(store));
            if (codec == null) throw new ArgumentNullException(nameof(codec));
            _store = store;
            _codec = codec;
        }

        public async Task<TValue> ReadAsync(string key, CancellationToken token)
        {
            return _codec.Decode(await _store.ReadAsync(key, token));
        }

        public async Task WriteAsync(string key, TValue value, TimeSpan lifetime, CancellationToken token)
        {
            await _store.WriteAsync(key, _codec.Encode(value), lifetime, token);
        }
    }
}
