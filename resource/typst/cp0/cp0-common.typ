
#let loc = $italic("Loc")$
#let var = $italic("Var")$
#let env = $italic("Env")$
#let store = $italic("Store")$
#let cont = $italic("Continuation")$
#let varflags = $italic("VarFlags")$
#let contextflags = $italic("ContextFlags")$
#let operand = $italic("Operand")$
#let identifier = $italic("Identifier")$
#let exp = $italic("Exp")$
#let ctx = $italic("Context")$
#let se = $italic("seq")$
#let unvisited = strong(`unvisited`)
#let reff = strong(`ref`)
#let assignf = strong(`assign`)
#let inlined = strong(`inlined`)
#let null = strong(`null`)
#let vars = strong(`Var`)
#let test = strong(`Test`)
#let opnd = strong(`Opnd`)
#let effect = strong(`Effect`)
#let value = strong(`Value`)
#let app = strong(`App`)
#let void = `void`
#let tru = `true`
#let fals = `false`

#let const(x) = [ `(const` #x`)` ]
#let ref(x) = [ `(ref` #x`)` ]
#let primref(x) = [ `(primref` #x`)` ]
#let iff(e1, e2, e3) = [ `(if` #e1 #e2 #e3`)` ]
#let seq(e1, e2) = [ `(seq` #e1 #e2`)` ]
#let assign(x, e) = [ `(assign` $#x$ #e`)` ]
#let lambda(x, e) = [ `(lambda (`$#x$`)` #e`)` ]
#let letrec(x1, e1, xn, en, e) = [ `(letrec ((`#x1 #e1`)` $dots$ `(`#xn #en`))` #e`)` ]
#let call(e0, e1) = [ `(call` #e0` `#e1`)` ]

#let E(content) = [
#show math.equation: set par(leading: .48em)
#content
]