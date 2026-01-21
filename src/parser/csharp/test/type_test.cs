namespace Ns0;

public enum Enum0
{
    A, B, C
}

namespace Ns1
{
    using ns3a = Ns3;

    namespace Ns2
    {
        public record Record2 { }
    }

    public struct Struct1<T> {
        public T Value;
    }

    public class Class1
    {
        public class ChildClass { }

        public ChildClass ChildClassField;

        public Struct1<ns3a::INterface3> SiblingStructProperty { get; }

        public Enum0[] ParentEnumArray;

        public Ns2.Record2 NieceRecordField;
    }
}

namespace Ns3
{
    public interface INterface3 { }
}