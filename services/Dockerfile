FROM ghcr.io/astral-sh/uv:python3.12-bookworm-slim AS builder

# Compile Python bytecode for faster application startup
ENV UV_COMPILE_BYTECODE=1
# Prevent uv from looking for global system Python installations
ENV UV_LINK_MODE=copy

WORKDIR /app

COPY pyproject.toml uv.lock ./


# Install external dependencies without installing the app package itself
RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --no-install-project --no-dev

COPY . .

RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --no-dev

FROM python:3.12-slim-bookworm

WORKDIR /app

# Ensure python logs are flushed immediately to stdout for Docker logging
ENV PYTHONUNBUFFERED=1

# Copy the fully prepared virtual environment from the builder stage
COPY --from=builder /app/.venv /app/.venv

# Copy your source code (FastAPI application code)
COPY . .

# Place the virtual environment's binaries directly into the PATH
ENV PATH="/app/.venv/bin:$PATH"

# Expose FastAPI's default port
EXPOSE 8000

# Run the FastAPI app using the standard production server wrapper
CMD ["uvicorn", "crawlers.api:app", "--port", "8000", "--host", "0.0.0.0"]