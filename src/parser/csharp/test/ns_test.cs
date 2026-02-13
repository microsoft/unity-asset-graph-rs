namespace L0;
using Ns0;
using ns0a = Ns0.InnerNs;
using static Ns0.InnerType;

namespace L1
{
    using Ns1;
    using ns1a = Ns1.InnerNs;
    using static Ns1.InnerType;

    namespace L2
    {
        using Ns2;
        using ns2a = Ns2.InnerNs;
        using static Ns2.InnerType;

        class Class2 : L3.Class3 {}
    }

    namespace L3
    {
        class Class3 {}
    }
}