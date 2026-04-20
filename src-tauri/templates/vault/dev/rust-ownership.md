---
type: til
created: 2026-04-21
tags:
  - rust
  - memory-safety
status: growing
sources:
  - https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html
---
# Rust Ownership

## 핵심 내용

Rust는 garbage collector 없이 메모리 안전을 달성하기 위해 **ownership** 규칙을 도입했다.

- 각 값에는 단 하나의 소유자(owner)가 있다.
- 소유자가 scope를 벗어나면 값이 drop된다.
- 소유권은 move(이동) 또는 borrow(참조)된다.

컴파일 타임에 규칙이 검증되므로 런타임 비용이 없다. C++의 RAII를 타입 시스템으로 강제한 것에 가깝다.

## 코드 예시

```rust
fn main() {
    let s = String::from("hello");
    takes_ownership(s);     // s의 소유권이 함수로 move됨
    // println!("{}", s);   // compile error: s는 이미 move됨

    let n = 5;
    makes_copy(n);          // i32는 Copy trait 구현 → 이동 아닌 복제
    println!("{}", n);      // OK
}

fn takes_ownership(s: String) { /* ... */ }
fn makes_copy(n: i32) { /* ... */ }
```

## 관련 노트

- [[building-a-second-brain]] — 노트 자체도 "누가 소유하는가"의 관점에서 비슷한 규칙
- [[seedling-llm-wiki]] — LLM이 ownership을 어떻게 추적하는지 흥미로운 질문
