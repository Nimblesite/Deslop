namespace Dispatch
{
    public sealed class AsyncFlow
    {
        public Runner RouteAsync(IAsyncJob inner)
        {
            if (inner == null) throw new ArgumentNullException(nameof(inner));
            return new Runner(
                (payload, context, cancellation, continueOnCapturedContext) => SharedEngine.ExecuteAsync(payload, context, cancellation, continueOnCapturedContext, this, inner),
                this,
                inner);
        }

        public Runner<T> RouteAsync<T>(IAsyncJob<T> inner)
        {
            if (inner == null) throw new ArgumentNullException(nameof(inner));
            return new Runner<T>(
                (payload, context, cancellation, continueOnCapturedContext) => SharedEngine.ExecuteAsync<T>(payload, context, cancellation, continueOnCapturedContext, this, inner),
                this,
                inner);
        }

        public Runner<T> RouteDefaultAsync<T>(IAsyncJob inner)
        {
            if (inner == null) throw new ArgumentNullException(nameof(inner));
            return new Runner<T>(
                (payload, context, cancellation, continueOnCapturedContext) => SharedEngine.ExecuteAsync<T>(payload, context, cancellation, continueOnCapturedContext, this, inner),
                this,
                inner);
        }
    }
}
