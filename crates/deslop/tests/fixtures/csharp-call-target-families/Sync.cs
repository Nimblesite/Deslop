using System;

namespace TwinPolicies
{
    public class SyncPolicySpecs
    {
        [Fact]
        public void NullPlainPolicyIsRejected()
        {
            RetryPolicy retry = Policy.Handle<Exception>().Retry(1);
            Action configure = () => retry.Wrap((Policy)null);
            configure.ShouldThrow<ArgumentNullException>().And.ParamName.Should().Be("innerPolicy");
        }

        [Fact]
        public void NullTypedPolicyIsRejected()
        {
            RetryPolicy retry = Policy.Handle<Exception>().Retry(1);
            Action configure = () => retry.Wrap<int>((Policy<int>)null);
            configure.ShouldThrow<ArgumentNullException>().And.ParamName.Should().Be("innerPolicy");
        }

        [Fact]
        public void NullPlainPolicyForTypedRetryIsRejected()
        {
            RetryPolicy<int> retry = Policy.HandleResult<int>(0).Retry(1);
            Action configure = () => retry.Wrap((Policy)null);
            configure.ShouldThrow<ArgumentNullException>().And.ParamName.Should().Be("innerPolicy");
        }

        [Fact]
        public void NullTypedPolicyForTypedRetryIsRejected()
        {
            RetryPolicy<int> retry = Policy.HandleResult<int>(0).Retry(1);
            Action configure = () => retry.Wrap((Policy<int>)null);
            configure.ShouldThrow<ArgumentNullException>().And.ParamName.Should().Be("innerPolicy");
        }

        [Fact]
        public void TwoPlainPoliciesKeepTheirOrder()
        {
            Policy first = Policy.NoOp();
            Policy second = Policy.NoOp();
            PolicyWrap wrap = first.Wrap(second);
            wrap.Outer.Should().BeSameAs(first);
            wrap.Inner.Should().BeSameAs(second);
        }

        [Fact]
        public void PlainOuterAndTypedInnerKeepTheirOrder()
        {
            Policy first = Policy.NoOp();
            Policy<int> second = Policy.NoOp<int>();
            PolicyWrap<int> wrap = first.Wrap(second);
            wrap.Outer.Should().BeSameAs(first);
            wrap.Inner.Should().BeSameAs(second);
        }

        [Fact]
        public void TypedOuterAndPlainInnerKeepTheirOrder()
        {
            Policy<int> first = Policy.NoOp<int>();
            Policy second = Policy.NoOp();
            PolicyWrap<int> wrap = first.Wrap(second);
            wrap.Outer.Should().BeSameAs(first);
            wrap.Inner.Should().BeSameAs(second);
        }

        [Fact]
        public void TwoTypedPoliciesKeepTheirOrder()
        {
            Policy<int> first = Policy.NoOp<int>();
            Policy<int> second = Policy.NoOp<int>();
            PolicyWrap<int> wrap = first.Wrap(second);
            wrap.Outer.Should().BeSameAs(first);
            wrap.Inner.Should().BeSameAs(second);
        }
    }
}
