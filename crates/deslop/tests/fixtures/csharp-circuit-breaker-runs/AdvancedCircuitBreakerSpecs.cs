using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using FluentAssertions;
using Polly.CircuitBreaker;
using Polly.Specs.Helpers;
using Polly.Utilities;
using Xunit;

using Scenario = Polly.Specs.Helpers.PolicyExtensions.ExceptionAndOrCancellationScenario;

namespace Polly.Specs.CircuitBreaker
{

    [Collection("SystemClockDependantCollection")]
    public class AdvancedCircuitBreakerSpecs : IDisposable
    {
        #region Configuration tests

        [Fact]
        public void Should_be_able_to_handle_a_duration_of_timespan_maxvalue()
        {
            CircuitBreakerPolicy breaker = Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(0.5, TimeSpan.FromSeconds(10), 4, TimeSpan.MaxValue);

            breaker.Invoking(x => x.RaiseException<DivideByZeroException>())
                .ShouldThrow<DivideByZeroException>();
        }

        [Fact]
        public void Should_throw_if_failure_threshold_is_zero()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(0, TimeSpan.FromSeconds(10), 4, TimeSpan.FromSeconds(30));

            action.ShouldThrow<ArgumentOutOfRangeException>()
                .And.ParamName.Should()
                .Be("failureThreshold");
        }

        [Fact]
        public void Should_throw_if_failure_threshold_is_less_than_zero()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(-0.5, TimeSpan.FromSeconds(10), 4, TimeSpan.FromSeconds(30));

            action.ShouldThrow<ArgumentOutOfRangeException>()
                .And.ParamName.Should()
                .Be("failureThreshold");
        }

        [Fact]
        public void Should_be_able_to_handle_a_failure_threshold_of_one()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(1.0, TimeSpan.FromSeconds(10), 4, TimeSpan.FromSeconds(30));

            action.ShouldNotThrow();
        }

        [Fact]
        public void Should_throw_if_failure_threshold_is_greater_than_one()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(1.01, TimeSpan.FromSeconds(10), 4, TimeSpan.FromSeconds(30));

            action.ShouldThrow<ArgumentOutOfRangeException>()
                .And.ParamName.Should()
                .Be("failureThreshold");
        }

        [Fact]
        public void Should_throw_if_timeslice_duration_is_less_than_resolution_of_circuit()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(
                    0.5, 
                    TimeSpan.FromMilliseconds(20).Add(TimeSpan.FromTicks(-1)), 
                    4, 
                    TimeSpan.FromSeconds(30));

            action.ShouldThrow<ArgumentOutOfRangeException>()
                .And.ParamName.Should()
                .Be("samplingDuration");
        }

        [Fact]
        public void Should_not_throw_if_timeslice_duration_is_resolution_of_circuit()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(0.5, TimeSpan.FromMilliseconds(20), 4, TimeSpan.FromSeconds(30));

            action.ShouldNotThrow<ArgumentOutOfRangeException>();
        }

        [Fact]
        public void Should_throw_if_minimum_throughput_is_one()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(0.5, TimeSpan.FromSeconds(10), 1, TimeSpan.FromSeconds(30));

            action.ShouldThrow<ArgumentOutOfRangeException>()
                .And.ParamName.Should()
                .Be("minimumThroughput");
        }

        [Fact]
        public void Should_throw_if_minimum_throughput_is_less_than_one()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(0.5, TimeSpan.FromSeconds(10), 0, TimeSpan.FromSeconds(30));

            action.ShouldThrow<ArgumentOutOfRangeException>()
                .And.ParamName.Should()
                .Be("minimumThroughput");
        }

        [Fact]
        public void Should_throw_if_duration_of_break_is_less_than_zero()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(0.5, TimeSpan.FromSeconds(10), 4, -TimeSpan.FromSeconds(1));

            action.ShouldThrow<ArgumentOutOfRangeException>()
                .And.ParamName.Should()
                .Be("durationOfBreak");
        }

        [Fact]
        public void Should_be_able_to_handle_a_duration_of_break_of_zero()
        {
            Action action = () => Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(0.5, TimeSpan.FromSeconds(10), 4, TimeSpan.Zero);

            action.ShouldNotThrow();
        }

        [Fact]
        public void Should_initialise_to_closed_state()
        {
            CircuitBreakerPolicy breaker = Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(0.5, TimeSpan.FromSeconds(10), 4, TimeSpan.FromSeconds(30));

            breaker.CircuitState.Should().Be(CircuitState.Closed);
        }

        #endregion

        #region Circuit-breaker threshold-to-break tests

        #region Tests that are independent from health metrics implementation

        // Tests on the AdvancedCircuitBreaker operation typically use a breaker: 
        // - with a failure threshold of >=50%, 
        // - and a throughput threshold of 4
        // - across a ten-second period.
        // These provide easy values for testing for failure and throughput thresholds each being met and non-met, in combination.

        [Fact]
        public void Should_not_open_circuit_if_failure_threshold_and_minimum_threshold_is_equalled_but_last_call_is_success()
        {
            var time = 1.January(2000);
            SystemClock.UtcNow = () => time;

            CircuitBreakerPolicy breaker = Policy
                .Handle<DivideByZeroException>()
                .AdvancedCircuitBreaker(
                    failureThreshold: 0.5,
                    samplingDuration: TimeSpan.FromSeconds(10),
                    minimumThroughput: 4,
                    durationOfBreak: TimeSpan.FromSeconds(30)
                );

            // Three of three actions in this test throw handled failures.
            breaker.Invoking(x => x.RaiseException<DivideByZeroException>())
                .ShouldThrow<DivideByZeroException>();
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            breaker.Invoking(x => x.RaiseException<DivideByZeroException>())
                .ShouldThrow<DivideByZeroException>();
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            breaker.Invoking(x => x.RaiseException<DivideByZeroException>())
                .ShouldThrow<DivideByZeroException>();
            breaker.CircuitState.Should().Be(CircuitState.Closed);
            // Failure threshold exceeded, but throughput threshold not yet.

            // Throughput threshold will be exceeded by the below successful call, but we never break on a successful call; hence don't break on this.
            breaker.Invoking(x => x.Execute(() => { }))
                .ShouldNotThrow();
            breaker.CircuitState.Should().Be(CircuitState.Closed);
            // No adjustment to SystemClock.UtcNow, so all exceptions were raised within same timeslice
        }

        [Fact]
        public void Should_not_open_circuit_if_exceptions_raised_are_not_one_of_the_specified_exceptions()
        {
            var time = 1.January(2000);
            SystemClock.UtcNow = () => time;

            CircuitBreakerPolicy breaker = Policy
                .Handle<DivideByZeroException>()
                .Or<ArgumentOutOfRangeException>()
                .AdvancedCircuitBreaker(
                    failureThreshold: 0.5,
                    samplingDuration: TimeSpan.FromSeconds(10),
                    minimumThroughput: 4,
                    durationOfBreak: TimeSpan.FromSeconds(30)
                );

            // Four of four actions in this test throw unhandled failures.
            breaker.Invoking(x => x.RaiseException<ArgumentNullException>())
                .ShouldThrow<ArgumentNullException>();
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            breaker.Invoking(x => x.RaiseException<ArgumentNullException>())
                .ShouldThrow<ArgumentNullException>();
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            breaker.Invoking(x => x.RaiseException<ArgumentNullException>())
                .ShouldThrow<ArgumentNullException>();
            breaker.CircuitState.Should().Be(CircuitState.Closed);

            breaker.Invoking(x => x.RaiseException<ArgumentNullException>())
                .ShouldThrow<ArgumentNullException>();
            breaker.CircuitState.Should().Be(CircuitState.Closed);
        }

    }
}
