namespace Dispatch
{
    public sealed class SyncFlow
    {
        public Runner Route(IJob inner)
        {
            if (inner == null) throw new ArgumentNullException(nameof(inner));
            return new Runner(
                (payload, context, cancellation) => SharedEngine.Execute(payload, context, cancellation, this, inner),
                this,
                inner);
        }

        public Runner<T> Route<T>(IJob<T> inner)
        {
            if (inner == null) throw new ArgumentNullException(nameof(inner));
            return new Runner<T>(
                (payload, context, cancellation) => SharedEngine.Execute<T>(payload, context, cancellation, this, inner),
                this,
                inner);
        }

        public Runner<T> RouteDefault<T>(IJob inner)
        {
            if (inner == null) throw new ArgumentNullException(nameof(inner));
            return new Runner<T>(
                (payload, context, cancellation) => SharedEngine.Execute<T>(payload, context, cancellation, this, inner),
                this,
                inner);
        }
    }
}
