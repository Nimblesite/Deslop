namespace AdapterChecks
{
    public class AsyncAdapterTests
    {
        [Fact]
        public void MissingBackendThrows()
        {
            Codec<object, Packed> codec = new Codec<object, Packed>(
                encode: value => new Packed(value),
                decode: packet => packet.Value
            );

            Action construct = () => new AsyncAdapter<Packed>(null, codec);

            construct.ShouldThrow<ArgumentNullException>()
                .And.ParamName.Should().Be("backend");
        }

        [Fact]
        public void MissingCodecThrows()
        {
            Action construct = () => new AsyncAdapter<object>(new StubStore().AsyncFor<object>(), null);

            construct.ShouldThrow<ArgumentNullException>()
                .And.ParamName.Should().Be("codec");
        }

        [Fact]
        public void FluentConstructionRejectsMissingCodec()
        {
            Action construct = () => new StubStore().AsyncFor<object>().WithCodec(null);

            construct.ShouldThrow<ArgumentNullException>()
                .And.ParamName.Should().Be("codec");
        }

        public async Task WriteAuditEntry(string message)
        {
            using var stream = await files.OpenAsync("audit.log");
            var payload = Encoding.UTF8.GetBytes(message);
            await stream.WriteAsync(payload);
        }
    }
}
