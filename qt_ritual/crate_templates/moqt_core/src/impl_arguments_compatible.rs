use crate::ArgumentsCompatible;

// generated with impl_arguments_compatible.py script
impl ArgumentsCompatible<()> for () {}

impl<T1> ArgumentsCompatible<()> for (T1,) {}
impl<T1> ArgumentsCompatible<(T1,)> for (T1,) {}

impl<T1, T2> ArgumentsCompatible<()> for (T1, T2) {}
impl<T1, T2> ArgumentsCompatible<(T1,)> for (T1, T2) {}
impl<T1, T2> ArgumentsCompatible<(T1, T2)> for (T1, T2) {}

impl<T1, T2, T3> ArgumentsCompatible<()> for (T1, T2, T3) {}
impl<T1, T2, T3> ArgumentsCompatible<(T1,)> for (T1, T2, T3) {}
impl<T1, T2, T3> ArgumentsCompatible<(T1, T2)> for (T1, T2, T3) {}
impl<T1, T2, T3> ArgumentsCompatible<(T1, T2, T3)> for (T1, T2, T3) {}

impl<T1, T2, T3, T4> ArgumentsCompatible<()> for (T1, T2, T3, T4) {}
impl<T1, T2, T3, T4> ArgumentsCompatible<(T1,)> for (T1, T2, T3, T4) {}
impl<T1, T2, T3, T4> ArgumentsCompatible<(T1, T2)> for (T1, T2, T3, T4) {}
impl<T1, T2, T3, T4> ArgumentsCompatible<(T1, T2, T3)> for (T1, T2, T3, T4) {}
impl<T1, T2, T3, T4> ArgumentsCompatible<(T1, T2, T3, T4)> for (T1, T2, T3, T4) {}

impl<T1, T2, T3, T4, T5> ArgumentsCompatible<()> for (T1, T2, T3, T4, T5) {}
impl<T1, T2, T3, T4, T5> ArgumentsCompatible<(T1,)> for (T1, T2, T3, T4, T5) {}
impl<T1, T2, T3, T4, T5> ArgumentsCompatible<(T1, T2)> for (T1, T2, T3, T4, T5) {}
impl<T1, T2, T3, T4, T5> ArgumentsCompatible<(T1, T2, T3)> for (T1, T2, T3, T4, T5) {}
impl<T1, T2, T3, T4, T5> ArgumentsCompatible<(T1, T2, T3, T4)> for (T1, T2, T3, T4, T5) {}
impl<T1, T2, T3, T4, T5> ArgumentsCompatible<(T1, T2, T3, T4, T5)> for (T1, T2, T3, T4, T5) {}

impl<T1, T2, T3, T4, T5, T6> ArgumentsCompatible<()> for (T1, T2, T3, T4, T5, T6) {}
impl<T1, T2, T3, T4, T5, T6> ArgumentsCompatible<(T1,)> for (T1, T2, T3, T4, T5, T6) {}
impl<T1, T2, T3, T4, T5, T6> ArgumentsCompatible<(T1, T2)> for (T1, T2, T3, T4, T5, T6) {}
impl<T1, T2, T3, T4, T5, T6> ArgumentsCompatible<(T1, T2, T3)> for (T1, T2, T3, T4, T5, T6) {}
impl<T1, T2, T3, T4, T5, T6> ArgumentsCompatible<(T1, T2, T3, T4)> for (T1, T2, T3, T4, T5, T6) {}
impl<T1, T2, T3, T4, T5, T6> ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6)
{
}
impl<T1, T2, T3, T4, T5, T6> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6)
{
}

impl<T1, T2, T3, T4, T5, T6, T7> ArgumentsCompatible<()> for (T1, T2, T3, T4, T5, T6, T7) {}
impl<T1, T2, T3, T4, T5, T6, T7> ArgumentsCompatible<(T1,)> for (T1, T2, T3, T4, T5, T6, T7) {}
impl<T1, T2, T3, T4, T5, T6, T7> ArgumentsCompatible<(T1, T2)> for (T1, T2, T3, T4, T5, T6, T7) {}
impl<T1, T2, T3, T4, T5, T6, T7> ArgumentsCompatible<(T1, T2, T3)>
    for (T1, T2, T3, T4, T5, T6, T7)
{
}
impl<T1, T2, T3, T4, T5, T6, T7> ArgumentsCompatible<(T1, T2, T3, T4)>
    for (T1, T2, T3, T4, T5, T6, T7)
{
}
impl<T1, T2, T3, T4, T5, T6, T7> ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6, T7)
{
}
impl<T1, T2, T3, T4, T5, T6, T7> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6, T7)
{
}
impl<T1, T2, T3, T4, T5, T6, T7> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (T1, T2, T3, T4, T5, T6, T7)
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<()> for (T1, T2, T3, T4, T5, T6, T7, T8) {}
impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<(T1,)>
    for (T1, T2, T3, T4, T5, T6, T7, T8)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<(T1, T2)>
    for (T1, T2, T3, T4, T5, T6, T7, T8)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<(T1, T2, T3)>
    for (T1, T2, T3, T4, T5, T6, T7, T8)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<(T1, T2, T3, T4)>
    for (T1, T2, T3, T4, T5, T6, T7, T8)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6, T7, T8)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6, T7, T8)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (T1, T2, T3, T4, T5, T6, T7, T8)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (T1, T2, T3, T4, T5, T6, T7, T8)
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<()>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1,)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1, T2)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1, T2, T3)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1, T2, T3, T4)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<()>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<(T1,)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<(T1, T2)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<(T1, T2, T3)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<(T1, T2, T3, T4)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> ArgumentsCompatible<()>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> ArgumentsCompatible<(T1,)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> ArgumentsCompatible<(T1, T2)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> ArgumentsCompatible<(T1, T2, T3)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> ArgumentsCompatible<(T1, T2, T3, T4)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> ArgumentsCompatible<()>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> ArgumentsCompatible<(T1,)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> ArgumentsCompatible<(T1, T2)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> ArgumentsCompatible<(T1, T2, T3)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> ArgumentsCompatible<(T1, T2, T3, T4)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13> ArgumentsCompatible<()>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13> ArgumentsCompatible<(T1,)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13> ArgumentsCompatible<(T1, T2)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13> ArgumentsCompatible<(T1, T2, T3)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13> ArgumentsCompatible<(T1, T2, T3, T4)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14> ArgumentsCompatible<()>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14> ArgumentsCompatible<(T1,)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14> ArgumentsCompatible<(T1, T2)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14> ArgumentsCompatible<(T1, T2, T3)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)>
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15> ArgumentsCompatible<()>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15> ArgumentsCompatible<(T1,)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15> ArgumentsCompatible<(T1, T2)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15>
    ArgumentsCompatible<(
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )
{
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16> ArgumentsCompatible<()>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1,)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
    )>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16>
    ArgumentsCompatible<(
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )>
    for (
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
    )
{
}
