#import "@preview/simplebnf:0.1.1": *
#import "cp0-common.typ": *

/*
ρ ∈ Env = Var → Var σ ∈ Store = (Locx → VarFlags) × (Locγ → ContextFlags) × (Loce → Exp ∪ {unvisited}) κ ∈ Continuation = Exp → Store → Exp s, r ∈ VarFlags ⊆ {ref, assign} ContextFlags ⊆ {inlined} op ∈ Operand ::= Opnd(Exp, Env , Loce) Var ::= Var(Identifier , Operand ∪ {null}, VarFlags, Locx) γ ∈ Context ::= Test | Effect | Value | App(Operand , Context, Locγ)  le ∈ Loce, lx ∈ Locx, lγ ∈ Locγ Locations
*/

#set page(height: auto, width: auto, margin: 1pt)
#bnf(
  Prod(
    $I$,
    delim: $:$,
    {
      Or[$exp arrow ctx arrow env arrow cont arrow store arrow exp$][]
    }
  ),
  Prod(
    $rho$,
    delim: $:$,
    annot: loc,
    {
      Or[$#var arrow #var$][]
    },
  ),
  Prod(
    $sigma$,
    delim: $:$,
    annot: store,
    {
      Or[$(loc_x arrow varflags) times (loc_y arrow contextflags) times (loc_e arrow exp union { unvisited })$][]
    },
  ),
  Prod(
    $k$,
    delim: $:$,
    annot: cont,
    {
      Or[$exp arrow store arrow exp$][]
    },
  ),
  Prod(
    $s, r$,
    delim: $subset.eq$,
    annot: varflags,
    {
      Or[${reff, assignf}$][]
    },
  ),
  Prod($$, delim: $subset.eq$, annot: contextflags, { Or[${inlined}$][] },),
  Prod(
    $italic("op")$,
    delim: $eq$,
    annot: operand,
    {
      Or[$opnd(exp, env, loc_e)$][]
    },
  ),
  Prod(
    $x$,
    delim: $eq$,
    annot: var,
    {
      Or[$vars(identifier, operand union { null }, varflags, loc_x)$][]
    },
  ),
  Prod(
    $gamma$,
    delim: $eq$,
    annot: ctx,
    {
      Or[#test][]
      Or[#effect][]
      Or[#value][]
      Or[$app(operand, ctx, loc_gamma)$][]
    },
  ),
)
