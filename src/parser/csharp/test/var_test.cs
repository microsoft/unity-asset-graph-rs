using X = Ns1.Class2;

namespace Ns0
{
    using System.Text;
    using Y = Ns1.Class3;

    public delegate X Delegate1(X first, Y second);

    public class Class1
    {
        public X Field;

        public Y Property { get; }

        public event Delegate1 Delegate;

        public string Repeat(int count)
        {
            X x = Delegate?.Invoke(Field, Property);
            using (int test = FakeClass.Get());
            using (StringBuilder sb = Main.StringBuilderCache.Get())
            {
                for (int i = 0; i < count; i++)
                {
                    sb.Append(x);
                }
                return sb.ToString();
            }
        }
    }
}

namespace Ns1
{
    public class Class2 {}
    public class Class3 {}
}
