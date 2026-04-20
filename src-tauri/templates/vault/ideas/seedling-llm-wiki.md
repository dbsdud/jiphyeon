---
type: idea
created: 2026-04-21
tags:
  - pkm
  - llm
status: seedling
priority: high
---
# Seedling: LLM Wiki

## 아이디어

Andrej Karpathy가 제안한 ["LLM wiki"](https://github.com/karpathy/llm-wiki) 아이디어를 구현해보자.

개인 위키가 실패하는 이유는 **지식이 부족해서**가 아니라 **유지 관리 비용이 가치보다 빠르게 증가**하기 때문이다. 교차 참조 업데이트, 요약 최신화, 일관성 유지 — 사람에겐 지루하지만 LLM은 지루함을 모른다.

## 핵심 전제

- 사람 — **무엇을 읽고 무엇을 물을지** 결정
- 볼트 — 개인 지식 저장소 (마크다운)
- LLM — 읽고, 정리하고, 잇고, 모순을 지적

## 질문들

- LLM이 **쓰기 권한**을 가질 때 어디까지 신뢰할 수 있나?
- 모순을 발견했을 때 **자동 해결 vs 사용자 확인** 어느 쪽?
- 여러 노트에 분산된 지식을 **언제** 종합(synthesize)해야 하나?

## 다음 단계

- [x] 볼트 기본 구조 설계 → [[2026-04-21-markdown-vault-stack]]
- [ ] Claude Code 스킬 구체화 (vault-new, vault-link, vault-synthesize)
- [ ] 모순 감지(vault-audit) 휴리스틱 실험

## 관련 노트

- [[building-a-second-brain]] — Forte의 CODE와 이 아이디어의 차이
- [[2026-04-21-markdown-vault-stack]] — 이 아이디어를 구현하기 위한 기술 선택
- [[rust-ownership]] — Rust로 앱을 만드는 것이 이 아이디어와 어울리는 이유(정적 배포)
