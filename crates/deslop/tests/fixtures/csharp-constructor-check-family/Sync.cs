namespace AdapterChecks
{
    public class BlockingAdapterTests
    {
        [Fact]
        public void MissingBackendThrows()
        {
            Codec<object, Packed> codec = new Codec<object, Packed>(
                encode: value => new Packed(value),
                decode: packet => packet.Value
            );

            Action construct = () => new BlockingAdapter<Packed>(null, codec);

            construct.ShouldThrow<ArgumentNullException>()
                .And.ParamName.Should().Be("backend");
        }

        [Fact]
        public void MissingCodecThrows()
        {
            Action construct = () => new BlockingAdapter<object>(new StubStore().For<object>(), null);

            construct.ShouldThrow<ArgumentNullException>()
                .And.ParamName.Should().Be("codec");
        }

        [Fact]
        public void FluentConstructionRejectsMissingCodec()
        {
            Action construct = () => new StubStore().For<object>().WithCodec(null);

            construct.ShouldThrow<ArgumentNullException>()
                .And.ParamName.Should().Be("codec");
        }

        public int CountExpiredEntries(int day)
        {
            var cutoff = DateTime.UtcNow.AddDays(-day);
            var expired = database.Query().Where(entry => entry.EndsAt < cutoff);
            return expired.Select(entry => entry.Key).Distinct().Count();
        }
    }
}
