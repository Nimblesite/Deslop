using System;

namespace TwinPolicies
{
    public class AsyncPolicySpecs
    {
        [Fact]
        public void NullPlainPolicyIsRejected()
        {
            RetryPolicy retry = Policy.Handle<Exception>().RetryAsync(1);
            Action configure = () => retry.WrapAsync((Policy)null);
            configure.ShouldThrow<ArgumentNullException>().And.ParamName.Should().Be("innerPolicy");
        }

        [Fact]
        public void NullTypedPolicyIsRejected()
        {
            RetryPolicy retry = Policy.Handle<Exception>().RetryAsync(1);
            Action configure = () => retry.WrapAsync<int>((Policy<int>)null);
            configure.ShouldThrow<ArgumentNullException>().And.ParamName.Should().Be("innerPolicy");
        }

        [Fact]
        public void NullPlainPolicyForTypedRetryIsRejected()
        {
            RetryPolicy<int> retry = Policy.HandleResult<int>(0).RetryAsync(1);
            Action configure = () => retry.WrapAsync((Policy)null);
            configure.ShouldThrow<ArgumentNullException>().And.ParamName.Should().Be("innerPolicy");
        }

        [Fact]
        public void NullTypedPolicyForTypedRetryIsRejected()
        {
            RetryPolicy<int> retry = Policy.HandleResult<int>(0).RetryAsync(1);
            Action configure = () => retry.WrapAsync((Policy<int>)null);
            configure.ShouldThrow<ArgumentNullException>().And.ParamName.Should().Be("innerPolicy");
        }

        [Fact]
        public void TwoPlainPoliciesKeepTheirOrder()
        {
            Policy first = Policy.NoOpAsync();
            Policy second = Policy.NoOpAsync();
            PolicyWrap wrap = first.WrapAsync(second);
            wrap.Outer.Should().BeSameAs(first);
            wrap.Inner.Should().BeSameAs(second);
        }

        [Fact]
        public void PlainOuterAndTypedInnerKeepTheirOrder()
        {
            Policy first = Policy.NoOpAsync();
            Policy<int> second = Policy.NoOpAsync<int>();
            PolicyWrap<int> wrap = first.WrapAsync(second);
            wrap.Outer.Should().BeSameAs(first);
            wrap.Inner.Should().BeSameAs(second);
        }

        [Fact]
        public void TypedOuterAndPlainInnerKeepTheirOrder()
        {
            Policy<int> first = Policy.NoOpAsync<int>();
            Policy second = Policy.NoOpAsync();
            PolicyWrap<int> wrap = first.WrapAsync(second);
            wrap.Outer.Should().BeSameAs(first);
            wrap.Inner.Should().BeSameAs(second);
        }

        [Fact]
        public void TwoTypedPoliciesKeepTheirOrder()
        {
            Policy<int> first = Policy.NoOpAsync<int>();
            Policy<int> second = Policy.NoOpAsync<int>();
            PolicyWrap<int> wrap = first.WrapAsync(second);
            wrap.Outer.Should().BeSameAs(first);
            wrap.Inner.Should().BeSameAs(second);
        }
    }
}
