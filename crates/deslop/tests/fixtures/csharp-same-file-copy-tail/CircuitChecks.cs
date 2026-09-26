// [PIPELINE-CLUSTER-EXACT-SCOPE] Three related circuit transitions; the latter two share an executable prefix.
public class CircuitChecks
{
        [Fact]
        public async Task Should_halfopen_circuit_after_the_specified_duration_has_passed()
        {
            var time = 1.January(2000);
            SystemClock.UtcNow = () => time;

            var durationOfBreak = TimeSpan.FromMinutes(1);

            CircuitBreakerPolicy<ResultPrimitive> breaker = Policy
                            .HandleResult(ResultPrimitive.Fault)
                            .CircuitBreakerAsync(2, durationOfBreak);

            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Open);

            // 2 exception or fault raised, circuit is now open
            breaker.Awaiting(async b => await b.RaiseResultSequenceAsync(ResultPrimitive.Fault))
                .ShouldThrow<BrokenCircuitException>();
            breaker.CircuitState.Should().Be(CircuitState.Open);

            SystemClock.UtcNow = () => time.Add(durationOfBreak);

            // duration has passed, circuit now half open
            breaker.CircuitState.Should().Be(CircuitState.HalfOpen);
            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
        }

        [Fact]
        public async Task Should_open_circuit_again_after_the_specified_duration_has_passed_if_the_next_call_raises_a_fault()
        {
            var time = 1.January(2000);
            SystemClock.UtcNow = () => time;

            var durationOfBreak = TimeSpan.FromMinutes(1);

            CircuitBreakerPolicy<ResultPrimitive> breaker = Policy
                            .HandleResult(ResultPrimitive.Fault)
                            .CircuitBreakerAsync(2, durationOfBreak);

            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Open);

            // 2 exception or fault raised, circuit is now open
            breaker.Awaiting(async b => await b.RaiseResultSequenceAsync(ResultPrimitive.Fault))
                  .ShouldThrow<BrokenCircuitException>();
            breaker.CircuitState.Should().Be(CircuitState.Open);

            SystemClock.UtcNow = () => time.Add(durationOfBreak);

            // duration has passed, circuit now half open
            breaker.CircuitState.Should().Be(CircuitState.HalfOpen);

            // first call after duration returns a fault, so circuit should break again
            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Open);
            breaker.Awaiting(async b => await b.RaiseResultSequenceAsync(ResultPrimitive.Fault))
                  .ShouldThrow<BrokenCircuitException>();

        }

        [Fact]
        public async Task Should_reset_circuit_after_the_specified_duration_has_passed_if_the_next_call_does_not_return_a_fault()
        {
            var time = 1.January(2000);
            SystemClock.UtcNow = () => time;

            var durationOfBreak = TimeSpan.FromMinutes(1);

            CircuitBreakerPolicy<ResultPrimitive> breaker = Policy
                            .HandleResult(ResultPrimitive.Fault)
                            .CircuitBreakerAsync(2, durationOfBreak);

            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Open);

            // 2 exception or fault raised, circuit is now open
            breaker.Awaiting(async b => await b.RaiseResultSequenceAsync(ResultPrimitive.Fault))
                  .ShouldThrow<BrokenCircuitException>();
            breaker.CircuitState.Should().Be(CircuitState.Open);

            SystemClock.UtcNow = () => time.Add(durationOfBreak);

            // duration has passed, circuit now half open
            breaker.CircuitState.Should().Be(CircuitState.HalfOpen);
            // first call after duration is successful, so circuit should reset
            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Good).ConfigureAwait(false))
                .Should().Be(ResultPrimitive.Good);
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            // circuit has been reset so should once again allow 2 faults to be raised before breaking
            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            (await breaker.RaiseResultSequenceAsync(ResultPrimitive.Fault).ConfigureAwait(false))
                  .Should().Be(ResultPrimitive.Fault);
            breaker.CircuitState.Should().Be(CircuitState.Open);

            breaker.Awaiting(async b => await b.RaiseResultSequenceAsync(ResultPrimitive.Fault))
                  .ShouldThrow<BrokenCircuitException>();
            breaker.CircuitState.Should().Be(CircuitState.Open);
        }
}
