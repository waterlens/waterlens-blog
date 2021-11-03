---
title: "Making Proofs in Coq"
date: 2021-11-03T11:58:34+08:00
draft: true
tags: [coq]
---

*This article should be used with a proof assistant for better understanding.*

### Enumeration

Let's start with the **enumeration**.
We will define a new **type** direction, which contains a set of values.
```coq
Inductive direction : Type :=
	| north
	| east
	| south
	| west.
```

If you want to operate the enumeration, you are likely to use the **pattern match**.
```coq
Definition opposite_direction (d: direction) : direction :=
	match d with
	| north => south
	| east  => west
	| south => north
	| west => east
	end.
```

We can expect such a function give us a 'proper' result. So we make the following **assertion**
```coq
(* 'Example' can be substituted by 'Lemma', 'Theorem' *)
Example opposite_east_is_west:
	opposite_direction east = west.
```

The assertion is intuitively correct. We can ask Coq to verify it (the following comes as our first proof) 
```coq
Proof.
	simpl. (* simplify *)
	reflexivity.
Qed.
```

####
