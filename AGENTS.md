## Software Design, Code Standards, Quality, and Style

* We must maintain a consistent style, with strong quality across the codebase.

### The GameEngine Standard Rust Software Design Specification

This section MUST be followed strictly.

1. Design patterns that identify common communication patterns among objects. By doing so, these patterns increase flexibility in carrying out communication.

* Use the Strategy Design Pattern &
* follow the Dependency Inversion Principle where practical and where that can be in line with the rest of the standards.

2. Write highly Idiomatic, exceptionally High quality Rust code

* Pay careful attention to the intent -- there may be several ways of doing something, but always do that which has the best of all of: performance/computational complexity, memory efficiency, likely binary size it will add, and auditability/readability, and is idiomatic.
* Leverage the Rust Compiler where possible, instead of fighting it. Prefer explicitly setting types for variables(e.g. `let x: u8 =`) rather then leaving it as `let x =` and letting the compiler infer it, even if it would work the other way.
* If you see code that does "clone to satisfy the borrow checker" -- you are seeing an anti-pattern, and must either point it out or resolve it. Obviously, do NOT write such code.
* Avoid Neutering Rust/Cargo -- For example, such a neutering would be `#![deny(warnings)]`. One must be intelligent and justified for such things.
* While Rust is Imperative, we certainly do make use of Functional Programming and Object-Oriented Programming principles where practical and intelligent to do so.
* Illegal states should be unrepresentable whenever practical

3. API truthfulness

* prefer sum types / typestate / smart constructors over post-hoc validation
* avoid booleans controlling large semantic mode switches when an enum is clearer
* Types must encode domain meaning whenever meaningful
* avoid raw String, u64, Vec<_> where a domain type is better
* use enums for modes and variants
* use newtypes for semantically distinct IDs/units
* Method signatures must reflect real ownership and mutability semantics
* do not hide expensive clones
* do not overuse `&mut self`; Be deliberate with `self` usage.
* consuming methods should consume for a reason

4. Public API Standards

* Public surface area must be intentionally minimal ; private by default
* expose only what users need (though in GameEngine, as scientific software, this may be quite broad)
* prefer a curated front-door API
* The public API must be stable in shape and naming
* consistent naming across modules
* no duplicate ways to do the same thing unless justified
* avoid leaking internal decomposition
* Configuration must be explicit and readable
* use config structs/builders for complex initialization, avoid long argument lists, and document defaults clearly

5. Software Design

* Follow the DRY principle: Do Not Repeat yourself. “Every piece of knowledge must have a single, unambiguous, authoritative representation within a system”
* Follow the Liskov Substitution Principle (LSP)
* Follow the Open/Closed Principle (OCP)
* Follow the Single Responsibility Principle (SRP)
* Follow the Keep It Simple, Stupid (KISS) Principle: Use Occams Razor to handle uncertainty/ambiguity. Simplicity is prerequisite for reliability.
* Follow the Law of Demeter.
* Design by contract -- Core invariants must be written down (not only in your head) module docs, type docs, extra pertinent comments on unsafe blocks, comments on algorithmic assumptions. No such invariant shall go untested.
* Tests should follow the semantic contract.
* In general, the library should be: truthful in its types, explicit in its effects and failure, minimal in its public API, composable in its abstractions, idiomatic in its surface, strict about invariants, predictable in behavior and cost
* Implement standard traits only when semantically correct: Clone, Copy, Eq, Ord, Hash, Debug, Default, Iterator, Display, etc., do not derive everything automatically just because it compiles.
* Documentation must explain both how and why, what this type means, what invariants it preserves, what guarantees the method gives, and what errors mean.
* Prefer generics over dynamic dispatch unless runtime polymorphism is needed, use dyn Trait only when actual runtime openness/heterogeneity is required
* do not crate-split reflexively; The Rust library is one crate, which should be **heavily** feature gated in architecture.
* Configuration should be explicit, performant, consistent, and with well documented defaults.
* Effects should be explicit and preferably at the edges
* Closed variation should use enums; open variation should use traits
* be idiomatic before being clever, unless this conflicts with other goals such as performance, memory efficiency, maintainability, extensibility, or binary size.

6. Handling of Unsafe Code

* Unsafe must have a proof obligation
* every unsafe block should explain: what invariant is required, why it holds, and what would break it
* Likewise, that must be fully tested.

7. Performance and Memory Efficiency

* Performance-sensitive behavior must be explicit
* document complexity where meaningful
* document allocation/cloning/caching behavior when users care
* do not accidentally make “cheap-looking” calls expensive

8. Error Handling

* Error types must be meaningful
* users should be able to understand and act on errors, and preserve context
* avoid collapsing everything into opaque strings too early
* All fallibility must be explicit: use Result, panic only for genuine bugs / impossible states / contract violation by programmer where that is truly the chosen policy

### Specification

* We must abide to included specification (which is usually latex) strictly and correctly.
* There may also be software design documents, which lay out the software design for feature(s)/aspects of the project, within scope these must be abided likewise. These may be tex or markdown.

### Summary

GameEngine MUST be held to the strictest of scientific and mathematical standards, and aligned with truth in general. We must abide to specification where applicable, and fix shortfalls to the standards laid out here, not work around, hide, or mask them. We must actively use the utmost high quality software design principles. It is an overall goal that, in line with the current Crate version, should must strictly match the .tex spec in spec/ -- i.e. for version 0.3.1 in line with spec/zerodotthreedotone.tex .
GameEngine aims for the utmost performance and memory efficiency, and it is actively an aim for the implementations which exist to themselves be done in the most performant and memory efficient manner, maximally so.


## Communication

Definition: user = the human Software Engineer you, as an Agent, are working with, who is presumably the main and sole developer of GameEngine.

The user WANTS you to think critically. They also want your responses/messages to them (like responses to the users questions, etc)  to be thorough: the user is an active developer that must understand everything going on in full, and you should never assume they will not understand something. You should explain concepts as clearly and correctly as you can -- if that is inherently hard to understand, that does not mean you omit it. The user necessitates high quality information.

### Non-Question Communication (User made a statement/declaration)

* In this case, in your mind, rephrase the users message as a question, then respond to it internally, reasoning through the implications of it.
* If anything seems, after that, like A) a bad idea, B) unnecessarily inefficient (in the computational complexity-theoretic notion), C) unnecessarily ugly, D)  Mathematically or scientifically wrong or incoherent -- then, double check that, and if you are confident, in proportion to that confidence profusely object to the statement in a solely reasonable manner, with a truth seeking intent.
* That should resolve ambiguity, accidental oversights, miscommunications, mistakes, and more. It is important that you explain why, exactly, and if relevant propose alternative ideas or suggestions as needed.
* If the users statement passes your audit, and is not just reasonable but you would agree it is a plain "Good Idea", then proceed with that as stated, and without objection(since in this case there is none).
* If the user is saying something vague, don't infer -- say exactly what they said was vague to them, let them know just in case they're unaware, but still specify the ambiguity, and aim for the user to resolve it, giving the necessary information. They do NOT want mistakes, which is the most important part.

### Questions

* In the case the user asks you a question, you must make sure you do not give them an answer based on a surface level reading/understanding, because that is not what they want.
* Double check your responses, and be mathematically meticulous where applicable.
* Be as objective and scientific as possible.
