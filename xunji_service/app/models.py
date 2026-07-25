"""FastAPI 输入输出模型，保持与网页端 Workout JSON 一致。"""

from typing import Any, Literal

from pydantic import BaseModel, Field, HttpUrl


class WorkoutSet(BaseModel):
    weightKg: float = Field(ge=0)
    reps: int = Field(ge=0)
    setNumber: int = Field(ge=1)


class WorkoutExercise(BaseModel):
    name: str = Field(min_length=1)
    sets: list[WorkoutSet]


class Workout(BaseModel):
    source: Literal["xunji"] = "xunji"
    date: str
    title: str
    durationMinutes: int = Field(ge=0)
    caloriesKcal: float = Field(ge=0)
    volumeKg: float = Field(ge=0)
    exercises: list[WorkoutExercise]


class ParseResponse(BaseModel):
    shareUrl: HttpUrl
    workout: Workout
    rawData: Any
    parser: Literal["embedded_json", "playwright", "dom"]

