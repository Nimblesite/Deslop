// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'models.dart';

Thing _$ThingFromJson(Map<String, dynamic> json) => Thing(
      id: json['id'] as String,
      name: json['name'] as String,
      count: (json['count'] as num).toInt(),
      active: json['active'] as bool,
    );

Map<String, dynamic> _$ThingToJson(Thing instance) => <String, dynamic>{
      'id': instance.id,
      'name': instance.name,
      'count': instance.count,
      'active': instance.active,
    };
