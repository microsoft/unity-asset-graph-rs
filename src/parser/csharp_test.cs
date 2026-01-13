using X;
using XYC = X.Y.Class;
using static X.Y.Class.StaticField;
namespace A;

namespace B {
    public class ClassB {
        public delegate void Delegate(int x);

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

        public void Method(in int a, string b, out int c)
        {
            
        }
    }

    namespace C {
        class ClassC { }
    }
}