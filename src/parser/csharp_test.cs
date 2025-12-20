using System;
using My.DifferentNamespace;

namespace My.Namespace {
    public class MyClass {
        public delegate void MyDelegate(int x);

        internal class UnderClass { }

        private static My.OtherNamespace.LocalizedString locstringNormal = LocStringCache.Get("NormalKey");

        private static LocalizedString locstringPrefixed = LocStringCache.Get(
            key: "PrefixedKey",
            formatArgs: "Some other text");

        private static LocalizedString locstringBad = LocStringCache.Get(someKey);

        private static LocalizedString locstringBadPrefix = LocStringCache.Get(key: someKey);

        public int MyProperty { get; set; }
    }

    struct MyStruct {
        public int X;
        public int Y;
    }

    enum MyEnum {
        First,
        Second,
        Third
    }

    interface IMyInterface {
        void DoSomething();
    }

    namespace InnerNamespace {
        class InnerClass { }
    }
}