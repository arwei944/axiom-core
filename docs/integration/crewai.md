# CrewAI Integration

This guide shows how to integrate Axiom cells as CrewAI tools and how to inspect crew execution through Axiom witnesses.

## Concept Mapping

| Axiom Concept | CrewAI Concept |
|---------------|----------------|
| `Cell` | Crew / Agent |
| `Signal` | Task / Output |
| `Witness` | Execution Audit |
| `Guard` | Constraint Validator |

## Setup

Create a CrewAI tool that delegates to an Axiom cell:

```python
from crewai.tools import BaseTool
import requests

class AxiomCellTool(BaseTool):
    name = "AxiomCell"
    description = "Send a task to an Axiom cell"

    def _run(self, task: str) -> str:
        resp = requests.post(
            "http://localhost:8080/cells/my-cell/send",
            json={"task": task},
        )
        return resp.text
```

## Example

```python
from crewai import Agent, Task, Crew

researcher = Agent(
    role="Researcher",
    goal="Gather information",
    backstory="You are a research assistant.",
    tools=[AxiomCellTool()],
)

task = Task(
    description="Send a greeting signal to the hello cell",
    agent=researcher,
)

crew = Crew(agents=[researcher], tasks=[task])
result = crew.kickoff()
```

## Observability

Inspect execution traces using Axiom witnesses:

```bash
axm witness inspect <witness_id>
```
