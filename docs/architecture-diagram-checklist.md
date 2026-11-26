# Architecture diagram checklist

Sources: C4 model review checklist (c4model.com/diagrams/checklist), ISO/IEC/IEEE 42010 (viewpoints, stakeholders, concerns), UML 2 component/deployment/sequence conventions, arc42, cloud reference styles (AWS/Azure: boundaries, grouped tiers, icons with labels), data-flow diagrams (trust boundaries).

## General
1. Title names what is shown. 2. Diagram type is stated (context, container, component, deployment, data flow, sequence). 3. Scope is stated (which system or part). 4. Key/legend explains colours, shapes, borders, line styles, arrowheads. 5. One abstraction level per diagram. 6. Intended audience/concern is clear (42010).

## Elements
7. Every element has a name. 8. Every element shows its type (service, store, queue, model, job, person, external system). 9. Every element says in one line what it does. 10. Technology is shown where it matters. 11. No unexplained acronyms. 12. Colour, shape, size and border carry a meaning that is in the key. 13. External systems and users are visibly outside the boundary. 14. Boundaries (system, trust, network, deployment) are drawn and named.

## Relationships
15. Every arrow has a label saying intent. 16. Label matches direction. 17. Protocol or technology shown on arrows that cross a boundary. 18. Sync vs async, request vs data flow distinguishable. 19. No crossing through unrelated elements; no orphan elements.

## Honesty and upkeep
20. Facts are traceable (each element links to evidence) and guesses are marked as such. 21. Predicted vs measured values are labelled. 22. Generated from code so it cannot drift; says when generated.

## Mechanically checkable from code
Title, type, scope, legend presence; names and types on every node; every edge has a label; no orphan nodes; every node has proof path:line; boundary groups exist; external systems detected from config.
