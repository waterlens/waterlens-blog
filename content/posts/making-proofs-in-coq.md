---
title: "Making Proofs in Coq"
date: 2021-11-03T11:58:34+08:00
draft: true
tags: [coq]
---

*This article should be used with a proof assistant for better understanding.*

### Enumeration

Let's start with the **enumeration**.
We will define a new **type** direction, which contains a set of values (constructors).
```coq
Inductive direction : Type :=
	| north
	| east
	| south
	| west.
```

If you want to operate the enumeration, you are likely to use the **pattern matching**.
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

#### Tuple

A tuple is a finite ordered sequence of elements.
In the coq language, we can use a contructor with some parameters to create a tuple type.

```coq
Inductive vec3 : Type :=
	| nat3 (x y z: nat).
```

The destruction of a tuple type can also be done by pattern matching

```coq
Definition is_zero_vec3 (v : vec3) : bool :=
	match v with
	| (nat3 0 0 0) => true
	| (nat3 _ _ _) => false
	end.

Example vec3_example1 :
	is_zero_vec3 (nat3 0 0 0) = true.
Proof.
	simpl.
	reflexivity.
Qed.

Example vec3_example2 :
	is_zero_vec3 (nat3 1 1 1) = false.
Proof.
	simpl.
	reflexivity.
Qed.
```
#### Natural Number

