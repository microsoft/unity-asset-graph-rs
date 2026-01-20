using X;
using XYC = X.Y.Class;
using static X.Y.Z.Class.StaticField;
using System.Text;
namespace A;

namespace B {
    public class ClassB {
        public delegate void Delegate(int x);

        public class InnerClass {}

        public int A;

        public event Delegate B;

        public int this[int x]
        {
            get
            {
                return StaticField[A];
            }
            set
            {
                A = value + x;
                B?.Invoke(A);
                XYC.StaticMethod(A);
            }
        }

        public int Ap
        {
            get => A;
            set => A = value;
        }

        public string Method(in int a, string b, out int c)
        {
            C.ClassC c = new();
            ClassB.InnerClass b = new();
            InnerClass bp = new();
            
            var poolobj = ObjectPool<InnerClass>.Get();
            using (StringBuilder sb = StringBuilderCache.Get())
            {
                for (int i = 0; i < 100; i++)
                {
                    sb.AppendFormat("({0})", i);
                }
                return sb.ToString();
            }
        }
    }

    namespace C {
        class ClassC { }
    }
}