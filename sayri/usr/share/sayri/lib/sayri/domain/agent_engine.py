"""ReAct Agent Orchestrator with Structured Tool Calling, Sandboxing, and Token-Efficiency."""

from __future__ import annotations

import json
import re
import threading
from typing import Any, Callable, Dict, List, Optional

from sayri import llm, paths, skills
from sayri.adapters.sandbox.executor import SandboxExecutor
from sayri.adapters.storage.sqlite_sessions import SQLiteSessionRepository
from sayri.domain.agent_creator import AgentCreator
from sayri.domain.models import (
    AgentProfile,
    Message,
    SandboxLevel,
    Session,
    ToolCall,
    ToolCallStatus,
)


class AgentEngine:
    """Orchestrates ReAct agent loop, tool calling, sandboxed execution, and memory persistence."""

    def __init__(
        self,
        storage: Optional[SQLiteSessionRepository] = None,
        sandbox: Optional[SandboxExecutor] = None,
    ) -> None:
        self.storage = storage or SQLiteSessionRepository()
        self.sandbox = sandbox or SandboxExecutor()
        self._active_queries: Dict[int, bool] = {}
        self._query_counter = 0

    def build_system_prompt(self, profile: AgentProfile) -> str:
        """Constructs a token-efficient system prompt with self-awareness of sub-agents, skills and sandboxing."""
        installed = skills.list_skills()
        skills_summary = ""
        if installed:
            items = [f"- {s['name']}: {s['description']}" for s in installed[:15]]
            skills_summary = "\nHABILIDADES INSTALADAS (Lee los detalles con la tool 'read_skill'):\n" + "\n".join(items)

        sandbox_info = f"Nivel de Aislamiento Activo: {profile.sandbox.level.value}."
        if profile.sandbox.level == SandboxLevel.LEVEL_0_NO_EXEC:
            sandbox_info += " (TIENES ESTRICTAMENTE PROHIBIDO EJECUTAR COMANDOS BASH/SISTEMA; eres un agente puramente conversacional de soporte)."

        base = (
            f"Eres Sayri, la asistente inteligente, orquestadora agéntica y copiloto de sistema operativo en Pulsar OS.\n"
            f"Perfil activo: {profile.name} (ID: {profile.id}). {sandbox_info}\n\n"
            "TUS CAPACIDADES Y PODERES EN PULSAR OS:\n"
            "1. Orquestación del Sistema: Puedes consultar archivos, abrir aplicaciones, modificar ajustes y ejecutar comandos con bash.\n"
            "2. Creación y Gestión de Subagentes: Puedes crear subagentes y configurarlos con diferentes modelos y niveles de sandbox (LEVEL_0_NO_EXEC, LEVEL_1_READONLY, LEVEL_2_ISOLATED_DEV, LEVEL_3_HOST_USER, LEVEL_4_HOST_ROOT). Si el usuario te pide crear un subagente (ej. para Discord, código o chat), puedes crearlo de inmediato.\n"
            "3. Creación y Gestión de Habilidades (Skills): Puedes crear y descargar habilidades de ClawHub (https://clawhub.ai) usando `sayri-skills install <nombre>` o creando plantillas en `~/.config/sayri/skills/`.\n"
            "4. Aislamiento y Sandboxing: El sistema ejecuta tus tareas dentro de contenedores ultraligeros Bubblewrap (bwrap) o con elevación Polkit (pkexec).\n"
            "5. Historial Persistente: Todas las conversaciones se guardan en SQLite y puedes consultarlas.\n"
            f"{skills_summary}\n\n"
            "REGLAS DE EJECUCIÓN (Crítico):\n"
            "1. Si necesitas realizar una acción o consultar datos, emite un bloque:\n"
            "```bash\n<comando>\n```\n"
            "2. Nunca digas 'No puedo crear subagentes' ni 'No tengo acceso a la terminal': SI TIENES acceso y puedes orquestar subagentes y herramientas dentro de tu sandbox.\n"
            "3. Responde siempre en español de forma natural, concisa y agradable (1 a 3 frases habladas para voz)."
        )
        return base

    def process_query(
        self,
        session_id: str,
        user_text: str,
        profile: AgentProfile,
        cfg: Any,
        on_delta: Callable[[str], None],
        on_done: Callable[[str], None],
        on_tool_start: Callable[[str], None],
        on_tool_finish: Callable[[str, str, int], None],
        on_error: Callable[[Exception], None],
    ) -> int:
        """Initiates a ReAct agent query in a background thread."""
        self._query_counter += 1
        query_id = self._query_counter
        self._active_queries[query_id] = True

        session = self.storage.get_session(session_id) or self.storage.create_session(
            agent_id=profile.id, title=user_text[:30]
        )

        user_msg = Message(role="user", content=user_text)
        self.storage.add_message(session.id, user_msg)

        # 1. Natural Language Subagent Intent Detection & Action
        clean_prompt = user_text.strip().lower()
        subagent_triggers = [
            "crea un subagente", "crear un subagente", "crear subagente", "crea subagente",
            "nuevo subagente", "configura un subagente", "configurar subagente", "quiero un subagente"
        ]
        if any(trig in clean_prompt for trig in subagent_triggers):
            ok, msg, created_profile = AgentCreator.create_agent_from_prompt(user_text)
            reply_msg = Message(role="assistant", content=msg)
            self.storage.add_message(session.id, reply_msg)
            on_delta(msg)
            on_done(msg)
            return query_id

        # 2. Async AI Title Generator
        if len(session.messages) <= 2 or session.title.startswith("Nueva Conversación") or session.title == user_text[:30]:
            self._generate_session_title_async(session.id, user_text, cfg)

        # Prepare messages payload
        messages = [{"role": "system", "content": self.build_system_prompt(profile)}]
        # Sliding context window (last 10 messages) for token efficiency
        for m in session.messages[-10:]:
            messages.append({"role": m.role, "content": m.content})
        messages.append({"role": "user", "content": user_text})

        threading.Thread(
            target=self._react_loop,
            args=(
                query_id,
                session_id,
                profile,
                messages,
                cfg,
                1,
                on_delta,
                on_done,
                on_tool_start,
                on_tool_finish,
                on_error,
            ),
            daemon=True,
        ).start()

        return query_id

    def _generate_session_title_async(self, session_id: str, first_query: str, cfg: Any) -> None:
        """Asynchronously calls LLM to generate a clean 3-4 word title for the session."""
        def _worker():
            try:
                base_url = cfg.get_string("provider", "base_url")
                api_key = cfg.get_string("provider", "api_key")
                model_name = cfg.get_string("provider", "model")
                if not base_url or not model_name:
                    return
                title_messages = [
                    {
                        "role": "system",
                        "content": "Eres un titulador de conversaciones. Genera un título ultra corto de 3 a 4 palabras en español que resuma la consulta del usuario. Responde ÚNICAMENTE con las 3-4 palabras, sin comillas, sin punto y sin explicaciones."
                    },
                    {"role": "user", "content": first_query[:120]}
                ]
                def _on_done(title_text: str):
                    clean = title_text.strip().strip('"\'').strip('.')
                    if clean and len(clean) >= 3 and not clean.startswith("HTTP"):
                        self.storage.update_session_title(session_id, clean[:36])
                        print(f"[AgentEngine] ✨ Auto-assigned session title: \"{clean[:36]}\"")

                llm.stream_chat(
                    base_url,
                    api_key,
                    model_name,
                    title_messages,
                    temperature=0.3,
                    max_tokens=15,
                    stream=False,
                    timeout=10,
                    on_delta=lambda _: None,
                    on_done=_on_done,
                    on_error=lambda _: None,
                )
            except Exception as exc:
                print(f"[AgentEngine] Title gen notice: {exc}")

        threading.Thread(target=_worker, daemon=True).start()

    def cancel_query(self, query_id: int) -> None:
        self._active_queries[query_id] = False

    def _react_loop(
        self,
        query_id: int,
        session_id: str,
        profile: AgentProfile,
        messages: List[Dict[str, Any]],
        cfg: Any,
        depth: int,
        on_delta: Callable[[str], None],
        on_done: Callable[[str], None],
        on_tool_start: Callable[[str], None],
        on_tool_finish: Callable[[str, str, int], None],
        on_error: Callable[[Exception], None],
    ) -> None:
        if not self._active_queries.get(query_id, False) or depth > 6:
            return

        base_url = profile.model.base_url or cfg.get_string("provider", "base_url")
        api_key = profile.model.api_key or cfg.get_string("provider", "api_key")
        model_name = profile.model.model_name
        if not model_name or model_name == "default":
            model_name = cfg.get_string("provider", "model")

        temperature = profile.model.temperature
        max_tokens = profile.model.max_tokens or cfg.get_int("provider", "max_tokens") or None

        current_full: List[str] = []

        def _handle_delta(delta: str) -> None:
            if not self._active_queries.get(query_id, False):
                return
            current_full.append(delta)
            on_delta(delta)

        def _handle_error(exc: Exception) -> None:
            if not self._active_queries.get(query_id, False):
                return
            on_error(exc)

        def _handle_done(full_text: str) -> None:
            if not self._active_queries.get(query_id, False):
                return

            # Check for bash commands in reply
            m = re.search(r"```(?:bash|sh)?\s*\n(.*?)\n```", full_text, re.DOTALL)
            if not m:
                m = re.search(r"<(?:bash|sh|tool)>(.*?)</(?:bash|sh|tool)>", full_text, re.DOTALL)

            if m and profile.sandbox.level != SandboxLevel.LEVEL_0_NO_EXEC and depth < 6:
                cmd = m.group(1).strip()
                if cmd:
                    on_tool_start(cmd)
                    retcode, output, duration = self.sandbox.execute(
                        cmd, profile.sandbox, agent_id=profile.id
                    )
                    on_tool_finish(cmd, output, retcode)

                    # Store tool execution
                    tc = ToolCall(
                        name="bash",
                        arguments={"command": cmd},
                        status=ToolCallStatus.SUCCESS if retcode == 0 else ToolCallStatus.FAILED,
                        output=output,
                        exit_code=retcode,
                        duration_ms=duration,
                    )
                    assistant_msg = Message(
                        role="assistant", content=full_text, tool_calls=[tc]
                    )
                    self.storage.add_message(session_id, assistant_msg)

                    # Followup
                    next_messages = list(messages)
                    next_messages.append({"role": "assistant", "content": full_text})
                    observation = (
                        f"[Tool Output - Código {retcode}]:\n{output}\n\n"
                        "Si la tarea está completada, responde de forma concisa y natural para el usuario. "
                        "Si requieres otro comando, emite un nuevo bloque ```bash."
                    )
                    next_messages.append({"role": "user", "content": observation})

                    self._react_loop(
                        query_id,
                        session_id,
                        profile,
                        next_messages,
                        cfg,
                        depth + 1,
                        on_delta,
                        on_done,
                        on_tool_start,
                        on_tool_finish,
                        on_error,
                    )
                    return

            # Final response
            final_msg = Message(role="assistant", content=full_text)
            self.storage.add_message(session_id, final_msg)
            on_done(full_text)

        llm.stream_chat(
            base_url,
            api_key,
            model_name,
            messages,
            temperature=temperature,
            max_tokens=max_tokens,
            stream=True,
            timeout=cfg.get_int("provider", "timeout"),
            on_delta=_handle_delta,
            on_done=_handle_done,
            on_error=_handle_error,
        )
