namespace CacheDemo
{
    public class SerializingProvider<TStored>
    {
        private readonly IStore<TStored> _store;
        private readonly ICodec<object, TStored> _codec;

        public SerializingProvider(IStore<TStored> store, ICodec<object, TStored> codec)
        {
            if (store == null) throw new ArgumentNullException(nameof(store));
            if (codec == null) throw new ArgumentNullException(nameof(codec));
            _store = store;
            _codec = codec;
        }

        public object Read(string key)
        {
            return _codec.Decode(_store.Read(key));
        }

        public void Write(string key, object value, TimeSpan lifetime)
        {
            _store.Write(key, _codec.Encode(value), lifetime);
        }
    }

    public class SerializingProvider<TValue, TStored>
    {
        private readonly IStore<TStored> _store;
        private readonly ICodec<TValue, TStored> _codec;

        public SerializingProvider(IStore<TStored> store, ICodec<TValue, TStored> codec)
        {
            if (store == null) throw new ArgumentNullException(nameof(store));
            if (codec == null) throw new ArgumentNullException(nameof(codec));
            _store = store;
            _codec = codec;
        }

        public TValue Read(string key)
        {
            return _codec.Decode(_store.Read(key));
        }

        public void Write(string key, TValue value, TimeSpan lifetime)
        {
            _store.Write(key, _codec.Encode(value), lifetime);
        }
    }
}
