from fastapi import APIRouter, HTTPException, Depends
from sqlmodel import Session
from app.db import get_session
from .models import User, UserCreate, UserPatch

users_router = APIRouter()


@users_router.get("/{id}", response_model=User)
def get_user(id: int, session: Session = Depends(get_session)):
    user = session.get(User, id)
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    return user


@users_router.post("/", response_model=User)
def post_user(user: UserCreate, session: Session = Depends(get_session)):
    db_user = User(**user.model_dump())
    session.add(db_user)
    session.commit()
    session.refresh(db_user)
    return db_user


@users_router.patch("/{id}", response_model=User)
def patch_user(id: int, fields: UserPatch, session: Session = Depends(get_session)):
    user = session.get(User, id)
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    for k, v in fields.model_dump(exclude_unset=True).items():
        setattr(user, k, v)
    session.add(user)
    session.commit()
    session.refresh(user)
    return user


# TODO: Implement a better response for delete
@users_router.delete("/{id}")
def delete_user(id: int, session: Session = Depends(get_session)):
    user = session.get(User, id)
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    session.delete(user)
    session.commit()
    return {"ok": True}
