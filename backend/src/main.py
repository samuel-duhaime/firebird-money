import uvicorn
from fastapi import FastAPI

app = FastAPI()


@app.get("/")
async def root():
    return {"message": "Hello World"}


def start():
    """Launched with `poetry run start`. Ensure this command is executed from the backend root directory."""
    uvicorn.run("src.main:app", host="0.0.0.0", port=8000, reload=True)
