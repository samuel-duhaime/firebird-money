from fastapi import APIRouter, HTTPException, Depends
from sqlmodel import Session
from app.db import get_session
from .models import Category, CategoryCreate, CategoryPatch

categories_router = APIRouter()


@categories_router.get("/{id}", response_model=Category)
def get_category(id: int, session: Session = Depends(get_session)):
    category = session.get(Category, id)
    if not category:
        raise HTTPException(status_code=404, detail="Category not found")
    return category


@categories_router.post("/", response_model=Category)
def post_category(category: CategoryCreate, session: Session = Depends(get_session)):
    db_category = Category(**category.model_dump())
    session.add(db_category)
    session.commit()
    session.refresh(db_category)
    return db_category


@categories_router.patch("/{id}", response_model=Category)
def patch_category(
    id: int, fields: CategoryPatch, session: Session = Depends(get_session)
):
    category = session.get(Category, id)
    if not category:
        raise HTTPException(status_code=404, detail="Category not found")
    for k, v in fields.model_dump(exclude_unset=True).items():
        setattr(category, k, v)
    session.add(category)
    session.commit()
    session.refresh(category)
    return category


@categories_router.delete("/{id}")
def delete_category(id: int, session: Session = Depends(get_session)):
    category = session.get(Category, id)
    if not category:
        raise HTTPException(status_code=404, detail="Category not found")
    session.delete(category)
    session.commit()
    return {"ok": True}
