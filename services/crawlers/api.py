import os
import asyncio
import logging
import sys
import time
from pathlib import Path
from dotenv import load_dotenv

load_dotenv()
os.environ["HF_TOKEN"] = "hf_xxxxxxxxxxxxxxxx"
os.environ["HF_HUB_OFFLINE"] = "1"

from fastapi import FastAPI, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from sentence_transformers import SentenceTransformer
from pydantic import BaseModel

ORIGINS = os.getenv("ORIGINS")

app = FastAPI()
log = logging.getLogger("uvicorn.error")

app.add_middleware(
    CORSMiddleware,
    allow_origins=ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],  # Or  specify: ["GET", "POST", "PUT", "DELETE"]
    allow_headers=["*"],
)

NAME = "BAAI/bge-base-en-v1.5"
try:
    MODEL = SentenceTransformer(NAME, local_files_only=True)
except:
    MODEL = SentenceTransformer(NAME)

SCRAPER_DIR = Path(__file__).parent
SCRAPER_SCRIPT = SCRAPER_DIR / "main.py"
COOLDOWN = 3600
COOLDOWN_CURRENCY = 86400

last_run: dict[str, float] = {}
in_flight: set[str] = set()
scrape_slots = asyncio.Semaphore(2)


class PayloadQuery(BaseModel):
    text: str | list[str]


async def embed_text(text: str | list[str]) -> list[float] | list[list[float]]:
    def _run_embedding():
        return MODEL.encode(text).tolist()

    embedding = await asyncio.to_thread(_run_embedding)
    return embedding


async def run_scraper(query: str):
    try:
        async with scrape_slots:
            proc = await asyncio.create_subprocess_exec(
                sys.executable,
                str(SCRAPER_SCRIPT),
                f"--search={query}",
                cwd=SCRAPER_DIR,
            )
            try:
                await asyncio.wait_for(proc.wait(), timeout=3000)
            except asyncio.TimeoutError:
                proc.kill()
                await proc.wait()
                log.warning("scraped time out for %r", query)
            else:
                if proc.returncode != 0:
                    log.warning(
                        "Scraped failed for %r (exit %s)", query, proc.returncode
                    )
    finally:
        in_flight.discard(query)


async def run_currency_scraper():
    try:
        async with scrape_slots:
            proc = await asyncio.create_subprocess_exec(
                sys.executable, "-m", "scrapy", "crawl", "curency_api", cwd=SCRAPER_DIR
            )
            try:
                await asyncio.wait_for(proc.wait(), timeout=3000)
            except asyncio.TimeoutError:
                proc.kill()
                await proc.wait()
                log.warning("scraped time out for currencies")
            else:
                if proc.returncode != 0:
                    log.warning(
                        "Scraped failed for currency (exit %s)", proc.returncode
                    )
    finally:
        log.info("Finished runing the currency spider")


@app.get("/health")
def health_check():
    return {"status": "ok"}


@app.post("/text-embedder")
async def embedded_text(payload: PayloadQuery):
    text_arr = await embed_text(payload.text)
    return {"data": text_arr}


@app.post("/run-scrapy")
async def run_scrapy(payload: PayloadQuery, background: BackgroundTasks):
    if not isinstance(payload.text, str):
        return {"status": "Invalid request payload"}
    query = payload.text.strip().lower()

    if time.monotonic() - last_run.get(query, -COOLDOWN) < COOLDOWN:
        return {"status": f"recently scraped {query}"}

    if query in in_flight:
        return {"status": "already runnig"}

    in_flight.add(query)
    last_run[query] = time.monotonic()

    background.add_task(run_scraper, query)
    return {"status": "Started to Run Scrapy crawler in background for products"}


@app.get("/update-currency")
async def run_update_currency(background: BackgroundTasks):
    current_time = time.monotonic()
    last_time_run = last_run.get("currency", -COOLDOWN_CURRENCY)
    if current_time - last_time_run < COOLDOWN_CURRENCY:
        return {"status": "recently scraped currency"}
    last_run["currency"] = current_time
    background.add_task(run_currency_scraper)
    return {"status": "Started to Run Scrapy crawler in background for currency"}
