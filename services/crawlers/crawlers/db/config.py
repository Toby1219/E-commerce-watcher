import os  
from tortoise import Tortoise  
from dotenv import load_dotenv

load_dotenv()

DB_USER = os.getenv("DB_USER")           # DB username, defaults to "postgres" locally
DB_PASSWORD = os.getenv("DB_PASSWORD")   # DB password
DB_HOST = os.getenv("DB_HOST", "localhost")          # where Postgres is running
DB_PORT = os.getenv("DB_PORT", "5433")                # default Postgres port
DB_NAME = os.getenv("DB_NAME", "e_watchdb")              # target database name


DB_URL = f"postgres://{DB_USER}:{DB_PASSWORD}@{DB_HOST}:{DB_PORT}/{DB_NAME}"

TORTOISE_ORM = {
    "connections": {
        "default": DB_URL,
    },
    "apps": {
        "models": {
            "models": ["crawlers.db.models"],
            "default_connection": "default",
        },
    },
}


async def init_db():
    await Tortoise.init(config=TORTOISE_ORM)
    await Tortoise.generate_schemas(safe=True)


async def close_db():
    """Call this on application shutdown to cleanly close DB connections."""
    await Tortoise.close_connections()


# # --- Example: wiring this into a plain asyncio script ---
# if __name__ == "__main__":
#     import asyncio  # needed to run the async init/close functions outside a web framework

#     async def main():
#         await init_db()   # open pool + create tables
#         print("Connected:", DB_URL)
#         await close_db()  # close pool cleanly before exiting

#     asyncio.run(main())