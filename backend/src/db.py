from sqlmodel import SQLModel, create_engine, Session
import os
from dotenv import load_dotenv
import sys

# Load environment variables from .env file
env_path = os.path.join(os.path.dirname(__file__), "..", ".env")
if not load_dotenv(dotenv_path=env_path):
    print(f"Warning: .env file not found at {env_path} or could not be loaded.")

# Build database URL from individual environment variables
POSTGRESQL_USER = os.getenv("POSTGRESQL_USER")
POSTGRESQL_PASSWORD = os.getenv("POSTGRESQL_PASSWORD")
POSTGRESQL_DB = os.getenv("POSTGRESQL_DB")
POSTGRESQL_HOST = os.getenv("POSTGRESQL_HOST", "localhost")
POSTGRESQL_PORT = os.getenv("POSTGRESQL_PORT", "5432")

# print(f"Connecting to PostgreSQL at {POSTGRESQL_HOST}:{POSTGRESQL_PORT} as user {POSTGRESQL_USER}")
missing_vars = []
for var in ["POSTGRESQL_USER", "POSTGRESQL_PASSWORD", "POSTGRESQL_DB"]:
    if not os.getenv(var):
        missing_vars.append(var)
if missing_vars:
    print(
        f"Warning: The following required environment variables are missing: {', '.join(missing_vars)}"
    )

if POSTGRESQL_USER and POSTGRESQL_PASSWORD and POSTGRESQL_DB:
    database_url = f"postgresql://{POSTGRESQL_USER}:{POSTGRESQL_PASSWORD}@{POSTGRESQL_HOST}:{POSTGRESQL_PORT}/{POSTGRESQL_DB}"
else:
    print(
        "Error: Required environment variables for PostgreSQL connection are not set."
    )
    sys.exit(1)  # Exit if DB config is missing


engine = create_engine(database_url, echo=True)


def get_session():
    with Session(engine) as session:
        yield session


def init_db():
    SQLModel.metadata.create_all(engine)
